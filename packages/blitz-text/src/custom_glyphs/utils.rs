//! Utility functions for custom glyph operations

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Fast color key hashing for glyph deduplication
#[inline(always)]
pub fn hash_color_key(color: Option<cosmyc_text::Color>) -> u32 {
    match color {
        Some(c) => {
            let mut hasher = DefaultHasher::new();
            c.r().hash(&mut hasher);
            c.g().hash(&mut hasher);
            c.b().hash(&mut hasher);
            c.a().hash(&mut hasher);
            hasher.finish() as u32
        }
        None => 0,
    }
}

/// Convert cosmyc_text::Color to glyphon::Color
#[inline(always)]
pub fn convert_cosmyc_color_to_glyphon(color: Option<cosmyc_text::Color>) -> Option<glyphon::Color> {
    color.map(|c| glyphon::Color::rgba(c.r(), c.g(), c.b(), c.a()))
}