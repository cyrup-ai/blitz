//! Custom glyph cache - simplified implementation
//! Core caching is now handled by goldylox in the text shaping layer

use std::sync::Arc;

use super::super::atlas::AtlasProcessor;
use super::super::registry::CustomGlyphRegistry;
use super::super::types::{CustomGlyphData, GlyphKey};

/// Custom glyph cache - simplified implementation
/// Core caching is handled by goldylox at the text shaping level
pub struct CustomGlyphCache {
    registry: Arc<CustomGlyphRegistry>,
    atlas_processor: Arc<AtlasProcessor>,
}

impl CustomGlyphCache {
    pub fn new(registry: Arc<CustomGlyphRegistry>, atlas_processor: Arc<AtlasProcessor>) -> Self {
        Self {
            registry,
            atlas_processor,
        }
    }

    pub fn get(&self, key: &GlyphKey) -> Option<CustomGlyphData> {
        // Direct lookup without caching - caching is handled at the text shaping level
        self.registry.get_glyph(key)
    }

    pub fn put(&mut self, key: GlyphKey, value: CustomGlyphData) -> Result<(), String> {
        // Direct storage without caching layer
        self.registry
            .register_glyph(key, value)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn clear(&mut self) {
        // Clear operation - simplified
        self.registry.clear();
    }

    pub fn size(&self) -> usize {
        self.registry.get_stats().total_glyphs as usize
    }
}
