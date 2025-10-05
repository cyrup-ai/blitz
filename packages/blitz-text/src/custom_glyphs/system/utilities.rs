//! Utility functions for custom glyph system
//!
//! This module provides utility functions for color conversion, hashing,
//! and integration with glyphon rasterization callbacks.

use glyphon::{ContentType, RasterizeCustomGlyphRequest, RasterizedCustomGlyph};

use super::super::atlas::AtlasProcessor;

/// Fast color key hashing for glyph deduplication
#[inline(always)]
pub fn hash_color_key(color: Option<cosmyc_text::Color>) -> u32 {
    match color {
        Some(c) => {
            let r = (c.r() as f32 * 255.0) as u32;
            let g = (c.g() as f32 * 255.0) as u32;
            let b = (c.b() as f32 * 255.0) as u32;
            let a = (c.a() as f32 * 255.0) as u32;
            (r << 24) | (g << 16) | (b << 8) | a
        }
        None => 0,
    }
}

/// Convert cosmyc_text::Color to glyphon::Color
#[inline(always)]
pub fn convert_cosmyc_color_to_glyphon(
    color: Option<cosmyc_text::Color>,
) -> Option<glyphon::Color> {
    color.map(|c| glyphon::Color::rgba(c.r(), c.g(), c.b(), c.a()))
}

/// Map Unicode codepoint to compact u16 ID (matches grid position logic in atlas.rs)
#[inline(always)]
pub fn codepoint_to_compact_id(codepoint: u32) -> Option<u16> {
    if AtlasProcessor::is_emoji_codepoint(codepoint) {
        // Emoji: U+1F600-1F64F maps to IDs 0-79
        Some((codepoint - 0x1F600) as u16)
    } else if AtlasProcessor::is_icon_codepoint(codepoint) {
        // Icons: U+E000-E0FF maps to IDs 256-511
        Some(((codepoint - 0xE000) + 256) as u16)
    } else {
        None
    }
}

/// Map compact u16 ID back to Unicode codepoint (inverse of codepoint_to_compact_id)
#[inline(always)]
pub fn compact_id_to_codepoint(id: u16) -> Option<u32> {
    if id < 256 {
        // IDs 0-79 are emoji (80-255 reserved for future emoji ranges)
        let codepoint = 0x1F600 + (id as u32);
        if codepoint <= 0x1F64F {
            Some(codepoint)
        } else {
            None // ID in reserved range
        }
    } else {
        // IDs 256-511 are icons
        let offset = (id - 256) as u32;
        let codepoint = 0xE000 + offset;
        if codepoint <= 0xE0FF {
            Some(codepoint)
        } else {
            None // Out of range
        }
    }
}

/// Rasterize custom glyph callback for glyphon integration
pub fn rasterize_custom_glyph(
    request: RasterizeCustomGlyphRequest,
) -> Option<RasterizedCustomGlyph> {
    // Map compact ID back to Unicode codepoint
    let codepoint = compact_id_to_codepoint(request.id)?;

    // Determine if emoji or icon
    let (glyph_data, content_type) = if AtlasProcessor::is_emoji_codepoint(codepoint) {
        // Extract emoji from embedded atlas
        let data = AtlasProcessor::extract_emoji(
            codepoint,
            request.width as u32,
            request.height as u32,
        )
        .ok()?;
        (data, ContentType::Color) // Emoji are full color RGBA
    } else if AtlasProcessor::is_icon_codepoint(codepoint) {
        // Extract icon from embedded atlas
        let data = AtlasProcessor::extract_icon(
            codepoint,
            request.width as u32,
            request.height as u32,
        )
        .ok()?;
        (data, ContentType::Mask) // Icons are monochrome masks
    } else {
        // Not a supported custom glyph
        return None;
    };

    Some(RasterizedCustomGlyph {
        data: glyph_data,
        content_type,
    })
}
