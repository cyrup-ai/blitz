//! Core custom glyph system implementation
//!
//! This module provides the main CustomGlyphSystem struct and its core functionality
//! including construction, configuration, and basic glyph operations.

use std::sync::Arc;

use super::super::atlas::AtlasProcessor;
use super::super::registry::CustomGlyphRegistry;
use super::super::types::{
    CustomGlyphData, CustomGlyphError, GlyphKey, GlyphSystemConfig, GlyphSystemStats,
};

/// Main custom glyph system integrating registry and GPU cache
pub struct CustomGlyphSystem {
    pub(super) registry: Arc<CustomGlyphRegistry>,
    pub(super) config: GlyphSystemConfig,
}

impl CustomGlyphSystem {
    /// Create new custom glyph system with configuration
    pub fn new(config: GlyphSystemConfig) -> Self {
        Self {
            registry: Arc::new(CustomGlyphRegistry::new()),
            config,
        }
    }

    /// Create custom glyph system with default configuration
    pub fn with_defaults() -> Self {
        Self::new(GlyphSystemConfig::default())
    }

    /// Get custom glyph by Unicode codepoint
    pub fn get_custom_glyph(
        &self,
        unicode_codepoint: u32,
        color_key: u32,
    ) -> Option<CustomGlyphData> {
        let key = GlyphKey::new(unicode_codepoint, 0, 0, color_key);
        self.registry.get_glyph(&key)
    }

    /// Get custom glyph by ID
    pub fn get_glyph_by_id(&self, id: glyphon::CustomGlyphId) -> Option<CustomGlyphData> {
        self.registry.get_glyph_by_id(id)
    }

    /// Check if glyph is registered
    pub fn has_glyph(&self, unicode_codepoint: u32, color_key: u32) -> bool {
        let key = GlyphKey::new(unicode_codepoint, 0, 0, color_key);
        self.registry.contains_glyph(&key)
    }

    /// Get custom glyphs for a text range
    pub fn get_glyphs_for_range(
        &self,
        text: &str,
        range: std::ops::Range<usize>,
    ) -> Result<Vec<glyphon::CustomGlyph>, CustomGlyphError> {
        let mut glyphs = Vec::new();

        // Extract text slice for the range
        let text_slice = text
            .get(range.clone())
            .ok_or(CustomGlyphError::InvalidRange)?;

        // Iterate through characters in the range
        for ch in text_slice.chars() {
            let codepoint = ch as u32;

            // Check if we have a custom glyph for this codepoint
            if let Some(glyph_data) = self.get_custom_glyph(codepoint, 0) {
                glyphs.push(glyph_data.glyph);
            }
        }

        Ok(glyphs)
    }

    /// Get system statistics
    pub fn get_stats(&self) -> GlyphSystemStats {
        let mut stats = self.registry.get_stats();

        // Update atlas utilization based on configuration
        let emoji_metadata = AtlasProcessor::emoji_atlas_metadata();
        let icon_metadata = AtlasProcessor::icon_atlas_metadata();
        let total_atlas_glyphs = emoji_metadata.total_glyphs() + icon_metadata.total_glyphs();

        stats.update_atlas_utilization(stats.total_glyphs, total_atlas_glyphs);
        stats
    }

    /// Clear all registered glyphs
    pub fn clear(&self) {
        self.registry.clear();
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f32 {
        self.registry.hit_rate()
    }

    /// Cleanup old unused glyphs
    pub fn cleanup_unused(&self, max_age_seconds: u32) -> usize {
        self.registry.cleanup_unused_glyphs(max_age_seconds)
    }

    /// Get most used glyphs
    pub fn get_most_used_glyphs(&self, limit: usize) -> Vec<(GlyphKey, CustomGlyphData)> {
        self.registry.get_glyphs_by_usage(true, limit)
    }

    /// Get least used glyphs
    pub fn get_least_used_glyphs(&self, limit: usize) -> Vec<(GlyphKey, CustomGlyphData)> {
        self.registry.get_glyphs_by_usage(false, limit)
    }

    /// Get configuration
    pub fn config(&self) -> &GlyphSystemConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: GlyphSystemConfig) {
        self.config = config;
    }
}

impl Default for CustomGlyphSystem {
    fn default() -> Self {
        Self::with_defaults()
    }
}
