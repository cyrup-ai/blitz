//! Custom glyph system module
//!
//! This module provides comprehensive custom glyph support with:
//! - Lock-free registry using ArcSwap for atomic operations
//! - Embedded emoji and icon atlas processing
//! - GPU cache integration for high-performance rendering
//! - Thread-local buffers for zero-allocation hot paths

pub mod atlas;
pub mod registry;
pub mod system;
pub mod types;

// Re-export main types and functionality
use std::cell::RefCell;
use std::sync::Arc;

pub use atlas::{
    calculate_emoji_texture_coords, calculate_icon_texture_coords, get_emoji_atlas_dimensions,
    get_icon_atlas_dimensions, AtlasProcessor,
};
pub use cosmyc_text::Buffer;
// Re-export glyphon types for consistency
pub use glyphon::{CustomGlyph, CustomGlyphId};
pub use registry::CustomGlyphRegistry;
pub use system::{hash_color_key, CustomGlyphCache, CustomGlyphSystem};
pub use types::AtlasCoords;
pub use types::{
    AtlasMetadata, CustomGlyphData, CustomGlyphError, GlyphKey, GlyphMetrics, GlyphSystemConfig,
    GlyphSystemStats,
};

use crate::types::ShapedRun;

thread_local! {
    static GLYPH_BUFFER: RefCell<Vec<CustomGlyph>> = RefCell::new(Vec::with_capacity(256));
    static GLYPHON_GLYPH_BUFFER: RefCell<Vec<glyphon::CustomGlyph>> = RefCell::new(Vec::with_capacity(256));
    static UNICODE_BUFFER: RefCell<Vec<char>> = RefCell::new(Vec::with_capacity(256));
    static ATLAS_COORD_BUFFER: RefCell<Vec<(u32, u32, u32, u32)>> = RefCell::new(Vec::with_capacity(256));
}

/// Global custom glyph system instance
static mut GLOBAL_GLYPH_SYSTEM: Option<Arc<CustomGlyphSystem>> = None;
static INIT_ONCE: std::sync::Once = std::sync::Once::new();

/// Initialize global custom glyph system
pub fn init_custom_glyph_system(config: GlyphSystemConfig) {
    INIT_ONCE.call_once(|| {
        let system = Arc::new(CustomGlyphSystem::new(config));
        unsafe {
            GLOBAL_GLYPH_SYSTEM = Some(system);
        }
    });
}

/// Get global custom glyph system
#[allow(static_mut_refs)]
pub fn get_global_glyph_system() -> Option<Arc<CustomGlyphSystem>> {
    unsafe { GLOBAL_GLYPH_SYSTEM.clone() }
}

/// Initialize with default configuration
pub fn init_with_defaults() {
    init_custom_glyph_system(GlyphSystemConfig::default());
}

/// Register a custom glyph globally
pub fn register_global_glyph(
    unicode_codepoint: u32,
    glyph_data: Vec<u8>,
    width: u32,
    height: u32,
    color_key: u32,
) -> Result<CustomGlyphId, CustomGlyphError> {
    let system = get_global_glyph_system().ok_or(CustomGlyphError::RegistryUnavailable)?;

    system.register_custom_glyph(unicode_codepoint, glyph_data, width, height, color_key)
}

/// Get custom glyph globally
pub fn get_global_glyph(unicode_codepoint: u32, color_key: u32) -> Option<CustomGlyphData> {
    let system = get_global_glyph_system()?;
    system.get_custom_glyph(unicode_codepoint, color_key)
}

/// Check if glyph exists globally
pub fn has_global_glyph(unicode_codepoint: u32, color_key: u32) -> bool {
    if let Some(system) = get_global_glyph_system() {
        system.has_glyph(unicode_codepoint, color_key)
    } else {
        false
    }
}

/// Get global system statistics
pub fn get_global_stats() -> Option<GlyphSystemStats> {
    let system = get_global_glyph_system()?;
    Some(system.get_stats())
}

/// Clear all global glyphs
pub fn clear_global_glyphs() {
    if let Some(system) = get_global_glyph_system() {
        system.clear();
    }
}

/// Utility functions for text system integration
pub mod utils {
    use super::*;

    /// Extract custom glyphs from shaped runs
    pub fn extract_custom_glyphs_from_runs(runs: &[ShapedRun]) -> Vec<char> {
        let mut custom_chars = Vec::new();

        for run in runs {
            for glyph in &run.glyphs {
                let codepoint = glyph.cluster as u32;
                if AtlasProcessor::is_emoji_codepoint(codepoint)
                    || AtlasProcessor::is_icon_codepoint(codepoint)
                {
                    if let Some(ch) = char::from_u32(codepoint) {
                        custom_chars.push(ch);
                    }
                }
            }
        }

        custom_chars
    }

    /// Pre-register glyphs from text content
    pub fn preregister_glyphs_from_text(
        text: &str,
        color_key: u32,
    ) -> Result<Vec<CustomGlyphId>, CustomGlyphError> {
        let system = get_global_glyph_system().ok_or(CustomGlyphError::RegistryUnavailable)?;

        let mut registered_ids = Vec::new();

        for ch in text.chars() {
            let codepoint = ch as u32;

            if AtlasProcessor::is_emoji_codepoint(codepoint) {
                if !system.has_glyph(codepoint, color_key) {
                    let id = system.register_emoji_glyph(codepoint)?;
                    registered_ids.push(id);
                }
            } else if AtlasProcessor::is_icon_codepoint(codepoint) {
                if !system.has_glyph(codepoint, color_key) {
                    let id = system.register_icon_glyph(codepoint)?;
                    registered_ids.push(id);
                }
            }
        }

        Ok(registered_ids)
    }

    /// Get glyph metrics for character
    pub fn get_glyph_metrics(character: char, color_key: u32) -> Option<GlyphMetrics> {
        let codepoint = character as u32;
        let glyph_data = get_global_glyph(codepoint, color_key)?;
        Some(glyph_data.metrics)
    }

    /// Check if character needs custom glyph rendering
    pub fn needs_custom_rendering(character: char) -> bool {
        let codepoint = character as u32;
        AtlasProcessor::is_emoji_codepoint(codepoint)
            || AtlasProcessor::is_icon_codepoint(codepoint)
    }

    /// Batch process text for custom glyphs
    pub fn process_text_for_custom_glyphs(
        text: &str,
        color_key: u32,
    ) -> Vec<(char, Option<CustomGlyphData>)> {
        text.chars()
            .map(|ch| {
                let codepoint = ch as u32;
                let glyph_data = if needs_custom_rendering(ch) {
                    get_global_glyph(codepoint, color_key)
                } else {
                    None
                };
                (ch, glyph_data)
            })
            .collect()
    }
}
