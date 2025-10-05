//! SIMD-optimized glyph flag computation with caching

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::atomic::Ordering;

use cosmyc_text::{CacheKeyFlags, LayoutGlyph, LayoutRun};

use crate::types::GlyphFlags;

use super::core::{CACHE_HIT_COUNT, CACHE_TOTAL_COUNT, GLYPH_PROPERTY_CACHE};

/// Fast glyph flags determination (optimized for hot path with SIMD and caching)
#[inline]
pub(super) fn determine_glyph_flags_fast(
    glyph: &LayoutGlyph,
    run: &LayoutRun,
) -> GlyphFlags {
    // Update cache statistics
    CACHE_TOTAL_COUNT.fetch_add(1, Ordering::Relaxed);

    // Fast cache lookup for common ASCII glyphs
    let cache_key = compute_cache_key(glyph);
    if let Some(cached_flags) = lookup_cached_flags(cache_key) {
        CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
        return cached_flags;
    }

    // SIMD-optimized flag computation for new glyphs
    let flags = compute_flags_simd(glyph, run);

    // Cache the result for future lookups
    cache_flags(cache_key, flags);

    flags
}

/// Compute cache key for glyph property lookup
#[inline]
fn compute_cache_key(glyph: &LayoutGlyph) -> u64 {
    // Create hash from glyph properties that affect flags
    let mut key = glyph.glyph_id as u64;
    key ^= (glyph.level.number() as u64) << 16;
    key ^= (glyph.cache_key_flags.bits() as u64) << 24;
    key ^= if glyph.start != glyph.end { 1 } else { 0 } << 32;
    key ^= if glyph.x_offset != 0.0 || glyph.y_offset != 0.0 {
        1
    } else {
        0
    } << 33;
    key
}

/// Fast cache lookup using atomic operations
#[inline]
fn lookup_cached_flags(cache_key: u64) -> Option<GlyphFlags> {
    // Simple cache using atomic compare-and-swap
    // For production, this would use a more sophisticated hash table
    let cached = GLYPH_PROPERTY_CACHE.load(Ordering::Relaxed);
    if cached as u64 == cache_key {
        // Cache hit - extract flags from upper bits
        Some(GlyphFlags::from_bits_truncate((cached >> 32) as u32))
    } else {
        None
    }
}

/// Cache computed flags atomically
#[inline]
fn cache_flags(cache_key: u64, flags: GlyphFlags) {
    // Store cache key in lower 32 bits, flags in upper 32 bits
    let cache_value = cache_key | ((flags.bits() as u64) << 32);
    GLYPH_PROPERTY_CACHE.store(cache_value as usize, Ordering::Relaxed);
}

/// SIMD-optimized flag computation
#[inline]
fn compute_flags_simd(glyph: &LayoutGlyph, run: &LayoutRun) -> GlyphFlags {
    // Pack glyph properties into SIMD vectors for parallel analysis
    #[cfg(target_arch = "x86_64")]
    unsafe {
        if is_x86_feature_detected!("sse2") {
            return compute_flags_sse2(glyph, run);
        }
    }

    // Fallback to scalar computation
    compute_flags_scalar(glyph, run)
}

/// SSE2-optimized flag computation
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn compute_flags_sse2(glyph: &LayoutGlyph, run: &LayoutRun) -> GlyphFlags {
    // Pack boolean conditions into vector for parallel evaluation
    let conditions = _mm_set_epi32(
        if glyph.start != glyph.end { 1 } else { 0 },
        if glyph.cache_key_flags.contains(CacheKeyFlags::UNSAFE_TO_BREAK) {
            1
        } else {
            0
        },
        if glyph.cache_key_flags.contains(CacheKeyFlags::CLUSTER_START) {
            1
        } else {
            0
        },
        if glyph.x_offset != 0.0 || glyph.y_offset != 0.0 {
            1
        } else {
            0
        },
    );

    // Use vector comparison for parallel evaluation
    let zero = _mm_setzero_si128();
    let comparison = _mm_cmpgt_epi32(conditions, zero);

    // Extract comparison results
    let mask = _mm_movemask_epi8(comparison);

    let mut flags = GlyphFlags::empty();

    // Set flags based on SIMD comparison results
    if mask & 0x000F != 0 {
        flags |= GlyphFlags::MARKS_ATTACHED;
    }
    if mask & 0x00F0 != 0 {
        flags |= GlyphFlags::IS_CLUSTER_START;
    }
    if mask & 0x0F00 != 0 {
        flags |= GlyphFlags::UNSAFE_TO_BREAK;
    }
    if mask & 0xF000 != 0 {
        flags |= GlyphFlags::CONTINUATION_CLUSTER;
    }

    // Additional script-specific analysis
    if is_cursive_script(glyph, run) {
        flags |= GlyphFlags::CURSIVE_CONNECTION;
    }

    if is_component_glyph(glyph) {
        flags |= GlyphFlags::COMPONENT_GLYPH;
    }

    if is_tatweel_safe(glyph, run) {
        flags |= GlyphFlags::SAFE_TO_INSERT_TATWEEL;
    }

    if is_unsafe_to_concat(glyph, run) {
        flags |= GlyphFlags::UNSAFE_TO_CONCAT;
    }

    flags
}

/// Scalar fallback flag computation
#[inline]
fn compute_flags_scalar(glyph: &LayoutGlyph, run: &LayoutRun) -> GlyphFlags {
    let mut flags = GlyphFlags::empty();

    // Basic cluster analysis
    if glyph.cache_key_flags.contains(CacheKeyFlags::CLUSTER_START) {
        flags |= GlyphFlags::IS_CLUSTER_START;
    } else {
        flags |= GlyphFlags::CONTINUATION_CLUSTER;
    }

    // Break safety analysis
    if glyph.cache_key_flags.contains(CacheKeyFlags::UNSAFE_TO_BREAK) {
        flags |= GlyphFlags::UNSAFE_TO_BREAK;
    }

    // Mark attachment detection
    if glyph.x_offset != 0.0 || glyph.y_offset != 0.0 {
        flags |= GlyphFlags::MARKS_ATTACHED;
    }

    // Script-specific analysis
    if is_cursive_script(glyph, run) {
        flags |= GlyphFlags::CURSIVE_CONNECTION;
    }

    if is_component_glyph(glyph) {
        flags |= GlyphFlags::COMPONENT_GLYPH;
    }

    if is_tatweel_safe(glyph, run) {
        flags |= GlyphFlags::SAFE_TO_INSERT_TATWEEL;
    }

    if is_unsafe_to_concat(glyph, run) {
        flags |= GlyphFlags::UNSAFE_TO_CONCAT;
    }

    flags
}

/// Check if glyph belongs to a cursive script
#[inline]
fn is_cursive_script(glyph: &LayoutGlyph, run: &LayoutRun) -> bool {
    use crate::features::FeatureLookup;
    
    // Get script from the run's metadata
    let script = run.script;
    FeatureLookup::is_cursive_script(script)
}

/// Check if glyph is a component of a larger character
#[inline]
fn is_component_glyph(glyph: &LayoutGlyph) -> bool {
    // Component glyphs typically span multiple character positions
    // or have specific positioning requirements
    glyph.start != glyph.end || (glyph.x_offset != 0.0 || glyph.y_offset != 0.0)
}

/// Check if tatweel insertion is safe at this position (Arabic)
#[inline]
fn is_tatweel_safe(glyph: &LayoutGlyph, run: &LayoutRun) -> bool {
    // Tatweel (kashida) can only be inserted between connecting Arabic letters
    // This is a simplified check - full implementation would analyze
    // the actual Arabic shaping context
    if !is_cursive_script(glyph, run) {
        return false;
    }

    // Don't insert tatweel at word boundaries or near marks
    !glyph.cache_key_flags.contains(CacheKeyFlags::UNSAFE_TO_BREAK)
        && glyph.x_offset == 0.0
        && glyph.y_offset == 0.0
}

/// Check if concatenation would be unsafe at this position
#[inline]
fn is_unsafe_to_concat(glyph: &LayoutGlyph, _run: &LayoutRun) -> bool {
    // Concatenation is unsafe if it would break shaping or positioning
    glyph.cache_key_flags.contains(CacheKeyFlags::UNSAFE_TO_BREAK)
        || (glyph.x_offset != 0.0 || glyph.y_offset != 0.0)
        || glyph.level.number() != unicode_bidi::Level::ltr().number()
}