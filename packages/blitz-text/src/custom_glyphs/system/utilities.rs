//! Utility functions for custom glyph system
//!
//! This module provides utility functions for color conversion, hashing,
//! and integration with glyphon rasterization callbacks.

use glyphon::RasterizeCustomGlyphRequest;

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

/// Rasterize custom glyph callback for glyphon integration
pub fn rasterize_custom_glyph(
    _request: RasterizeCustomGlyphRequest,
) -> Option<glyphon::CustomGlyph> {
    // This would integrate with the custom glyph system
    // For now, return None to indicate no custom rasterization
    None
}
