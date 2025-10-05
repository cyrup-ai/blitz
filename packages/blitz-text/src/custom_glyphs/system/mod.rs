//! Custom glyph system integration
//!
//! This module provides the main system interface that integrates
//! registry, atlas processing, and GPU cache management.

mod cache;
mod core;
mod registration;
mod utilities;

pub use core::CustomGlyphSystem;

pub use cache::CustomGlyphCache;
pub use utilities::{
    codepoint_to_compact_id, compact_id_to_codepoint, convert_cosmyc_color_to_glyphon,
    hash_color_key, rasterize_custom_glyph,
};
