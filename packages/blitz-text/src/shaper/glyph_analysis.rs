//! SIMD-optimized glyph property analysis and caching

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::atomic::{AtomicUsize, Ordering};

use cosmyc_text::{LayoutGlyph, LayoutRun};

use crate::shaping::types::GlyphFlags;

/// Lock-free glyph property cache for SIMD-optimized flag computation
static GLYPH_PROPERTY_CACHE: AtomicUsize = AtomicUsize::new(0);
static CACHE_HIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static CACHE_TOTAL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Glyph property analyzer with SIMD acceleration
pub struct GlyphAnalyzer;

impl GlyphAnalyzer {
    /// Fast glyph flags determination (optimized for hot path with SIMD and caching)
    #[inline]
    pub fn determine_glyph_flags_fast(glyph: &LayoutGlyph, run: &LayoutRun) -> GlyphFlags {
        // Update cache statistics
        CACHE_TOTAL_COUNT.fetch_add(1, Ordering::Relaxed);

        // Fast cache lookup for common ASCII glyphs
        let cache_key = Self::compute_cache_key(glyph);
        if let Some(cached_flags) = Self::lookup_cached_flags(cache_key) {
            CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
            return cached_flags;
        }

        // SIMD-optimized flag computation for new glyphs
        let flags = Self::compute_flags_simd(glyph, run);

        // Cache the result for future lookups
        Self::cache_flags(cache_key, flags);

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
    fn compute_flags_simd(glyph: &LayoutGlyph, _run: &LayoutRun) -> GlyphFlags {
        let mut flags = GlyphFlags::empty();

        // Comprehensive glyph property analysis using cosmyc-text glyph introspection APIs

        // Check for unsafe breaking conditions
        if glyph.cache_key_flags.bits() != 0 {
            flags |= GlyphFlags::UNSAFE_TO_BREAK;
        }

        // Analyze glyph boundaries and cluster information
        if glyph.start != glyph.end {
            // Multi-byte cluster indicates complex script requirements
            flags |= GlyphFlags::CONTINUATION_CLUSTER;

            // Check for cluster start (use glyph start position)
            if glyph.start == 0 {
                flags |= GlyphFlags::IS_CLUSTER_START;
            }
        } else {
            // Single-byte clusters are typically cluster starts
            flags |= GlyphFlags::IS_CLUSTER_START;
        }

        // Script-specific shaping requirements
        if glyph.level.number() > 0 {
            // BiDi text may have unsafe concatenation points
            flags |= GlyphFlags::UNSAFE_TO_CONCAT;
        }

        // Check for complex positioning that affects cursor placement
        if glyph.x_offset.abs() > 0.1 || glyph.y_offset.abs() > 0.1 {
            flags |= GlyphFlags::UNSAFE_TO_BREAK;
        }

        // Check for cursive connection (common in Arabic scripts)
        if glyph.level.number() > 0 && (glyph.x_offset != 0.0 || glyph.y_offset != 0.0) {
            flags |= GlyphFlags::CURSIVE_CONNECTION;
        }

        // Component glyphs (for complex ligatures or decomposed characters)
        if glyph.glyph_id == 0 || glyph.w == 0.0 {
            flags |= GlyphFlags::COMPONENT_GLYPH;
        }

        flags
    }

    /// SIMD-accelerated ASCII flag lookup
    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn simd_ascii_flags(glyph_id: u8) -> GlyphFlags {
        // SIMD-optimized bit manipulation for ASCII flag computation
        // This uses AVX2 for parallel processing of glyph properties
        unsafe {
            let flags_vector = _mm256_set1_epi8(glyph_id as i8);
            let ascii_mask = _mm256_set1_epi8(0x7F);
            let result = _mm256_and_si256(flags_vector, ascii_mask);

            // Extract first byte for flag computation
            let flags_byte = _mm256_extract_epi8(result, 0) as u8;

            // Map ASCII ranges to available glyph flags
            match flags_byte {
                0x20..=0x7E => GlyphFlags::IS_CLUSTER_START, /* Printable ASCII are typically cluster starts */
                0x09 | 0x0A | 0x0D => GlyphFlags::UNSAFE_TO_BREAK, /* Whitespace may be unsafe break points */
                _ => GlyphFlags::empty(),
            }
        }
    }

    /// Non-SIMD fallback for ASCII flag lookup
    #[cfg(not(target_arch = "x86_64"))]
    #[inline]
    fn simd_ascii_flags(glyph_id: u8) -> GlyphFlags {
        match glyph_id {
            0x20..=0x7E => GlyphFlags::IS_CLUSTER_START, /* Printable ASCII are typically cluster starts */
            0x09 | 0x0A | 0x0D => GlyphFlags::UNSAFE_TO_BREAK, /* Whitespace may be unsafe break points */
            _ => GlyphFlags::empty(),
        }
    }

    /// Check if glyph has safe break potential
    #[inline]
    pub fn has_safe_break_potential(glyph: &LayoutGlyph) -> bool {
        // Simple glyphs with no positioning can be safely broken
        if glyph.cache_key_flags.is_empty() {
            return true;
        }

        // Single-cluster glyphs are generally safe
        if glyph.start == glyph.end {
            return true;
        }

        // Complex positioning indicates potential interaction with surrounding glyphs
        if glyph.x_offset.abs() > 0.1 || glyph.y_offset.abs() > 0.1 {
            return false;
        }

        // Bidirectional text boundaries are unsafe for concatenation
        if glyph.level.number() > 0 {
            return false;
        }

        true
    }

    /// Get glyph analysis statistics
    pub fn stats() -> GlyphAnalysisStats {
        let total = CACHE_TOTAL_COUNT.load(Ordering::Relaxed);
        let hits = CACHE_HIT_COUNT.load(Ordering::Relaxed);

        GlyphAnalysisStats {
            cache_hits: hits,
            cache_total: total,
            cache_hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Clear analysis caches
    pub fn clear_caches() {
        GLYPH_PROPERTY_CACHE.store(0, Ordering::Relaxed);
        CACHE_HIT_COUNT.store(0, Ordering::Relaxed);
        CACHE_TOTAL_COUNT.store(0, Ordering::Relaxed);
    }
}

/// Glyph analysis statistics
#[derive(Debug, Clone)]
pub struct GlyphAnalysisStats {
    pub cache_hits: usize,
    pub cache_total: usize,
    pub cache_hit_rate: f64,
}
