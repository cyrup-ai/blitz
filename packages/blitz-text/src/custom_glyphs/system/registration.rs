//! Glyph registration functionality
//!
//! This module provides methods for registering custom glyphs, including
//! individual registration, batch registration, and atlas-based registration.

use glyphon::{CustomGlyph, CustomGlyphId};

use super::super::atlas::AtlasProcessor;
use super::super::types::{AtlasCoords, CustomGlyphData, CustomGlyphError, GlyphKey, GlyphMetrics};
use super::core::CustomGlyphSystem;

impl CustomGlyphSystem {
    /// Register a custom glyph in the system
    pub fn register_custom_glyph(
        &self,
        unicode_codepoint: u32,
        _glyph_data: Vec<u8>,
        width: u32,
        height: u32,
        color_key: u32,
    ) -> Result<CustomGlyphId, CustomGlyphError> {
        // Create glyph key
        let key = GlyphKey::new(unicode_codepoint, 0, 0, color_key);

        // Get atlas coordinates
        let atlas_coords =
            AtlasProcessor::get_coords_for_codepoint(unicode_codepoint).unwrap_or(AtlasCoords {
                x: 0,
                y: 0,
                width: width as u16,
                height: height as u16,
            });

        // Create glyph metrics
        let metrics = GlyphMetrics::default_for_size(height as f32);

        // Create custom glyph
        let custom_glyph = CustomGlyph {
            id: 0 as CustomGlyphId, // Will be assigned by registry
            left: 0.0,
            top: 0.0,
            width: width as f32,
            height: height as f32,
            color: Some(cosmyc_text::Color::rgba(255, 255, 255, 255)), // Default white color
            snap_to_physical_pixel: false,
            metadata: 0,
        };

        // Create glyph data
        let glyph_data = CustomGlyphData::new(custom_glyph, atlas_coords, metrics);

        // Register in registry
        self.registry.register_glyph(key, glyph_data)
    }

    /// Register emoji glyph from embedded atlas
    pub fn register_emoji_glyph(&self, codepoint: u32) -> Result<CustomGlyphId, CustomGlyphError> {
        if !AtlasProcessor::is_emoji_codepoint(codepoint) {
            return Err(CustomGlyphError::CoordinateCalculationFailed(codepoint));
        }

        let atlas_metadata = AtlasProcessor::emoji_atlas_metadata();
        let glyph_data = AtlasProcessor::extract_emoji(
            codepoint,
            atlas_metadata.glyph_width,
            atlas_metadata.glyph_height,
        )?;

        self.register_custom_glyph(
            codepoint,
            glyph_data,
            atlas_metadata.glyph_width,
            atlas_metadata.glyph_height,
            0, // Default color key
        )
    }

    /// Register icon glyph from embedded atlas
    pub fn register_icon_glyph(&self, codepoint: u32) -> Result<CustomGlyphId, CustomGlyphError> {
        if !AtlasProcessor::is_icon_codepoint(codepoint) {
            return Err(CustomGlyphError::CoordinateCalculationFailed(codepoint));
        }

        let atlas_metadata = AtlasProcessor::icon_atlas_metadata();
        let glyph_data = AtlasProcessor::extract_icon(
            codepoint,
            atlas_metadata.glyph_width,
            atlas_metadata.glyph_height,
        )?;

        self.register_custom_glyph(
            codepoint,
            glyph_data,
            atlas_metadata.glyph_width,
            atlas_metadata.glyph_height,
            0, // Default color key
        )
    }

    /// Batch register multiple glyphs
    pub fn batch_register_glyphs(
        &self,
        glyphs: Vec<(u32, Vec<u8>, u32, u32, u32)>, // (codepoint, data, width, height, color_key)
    ) -> Result<Vec<CustomGlyphId>, CustomGlyphError> {
        let mut glyph_data_vec = Vec::with_capacity(glyphs.len());

        for (codepoint, _data, width, height, color_key) in glyphs {
            let key = GlyphKey::new(codepoint, 0, 0, color_key);

            let atlas_coords =
                AtlasProcessor::get_coords_for_codepoint(codepoint).unwrap_or(AtlasCoords {
                    x: 0,
                    y: 0,
                    width: width as u16,
                    height: height as u16,
                });

            let metrics = GlyphMetrics::default_for_size(height as f32);

            let custom_glyph = CustomGlyph {
                id: 0 as CustomGlyphId, // Will be assigned by registry
                left: 0.0,
                top: 0.0,
                width: width as f32,
                height: height as f32,
                color: Some(cosmyc_text::Color::rgba(255, 255, 255, 255)), // Default white color
                snap_to_physical_pixel: false,
                metadata: 0,
            };

            let glyph_data = CustomGlyphData::new(custom_glyph, atlas_coords, metrics);
            glyph_data_vec.push((key, glyph_data));
        }

        self.registry.batch_register_glyphs(glyph_data_vec)
    }
}
