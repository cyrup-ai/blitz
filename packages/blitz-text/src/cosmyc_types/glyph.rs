//! Glyph information and utilities
//!
//! This module provides comprehensive glyph information extraction and conversion
//! utilities for working with layout glyphs and physical glyphs.

use cosmyc_text::{CacheKeyFlags, Color, LayoutGlyph, PhysicalGlyph};

/// Comprehensive glyph information extracted from LayoutGlyph
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    pub start: usize,
    pub end: usize,
    pub font_size: f32,
    pub font_weight: cosmyc_text::fontdb::Weight,
    pub line_height_opt: Option<f32>,
    pub font_id: cosmyc_text::fontdb::ID,
    pub glyph_id: u16,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub level: unicode_bidi::Level,
    pub x_offset: f32,
    pub y_offset: f32,
    pub color_opt: Option<Color>,
    pub metadata: usize,
    pub cache_key_flags: CacheKeyFlags,
}

impl GlyphInfo {
    /// Extract glyph information from LayoutGlyph
    pub fn from_layout_glyph(glyph: &LayoutGlyph) -> Self {
        Self {
            start: glyph.start,
            end: glyph.end,
            font_size: glyph.font_size,
            font_weight: glyph.font_weight,
            line_height_opt: glyph.line_height_opt,
            font_id: glyph.font_id,
            glyph_id: glyph.glyph_id,
            x: glyph.x,
            y: glyph.y,
            w: glyph.w,
            level: glyph.level,
            x_offset: glyph.x_offset,
            y_offset: glyph.y_offset,
            color_opt: glyph.color_opt,
            metadata: glyph.metadata,
            cache_key_flags: glyph.cache_key_flags,
        }
    }

    /// Convert to physical glyph for rendering
    pub fn to_physical(&self, offset: (f32, f32), scale: f32) -> PhysicalGlyph {
        // Create a temporary LayoutGlyph to call physical() method
        let layout_glyph = LayoutGlyph {
            start: self.start,
            end: self.end,
            font_size: self.font_size,
            font_weight: self.font_weight,
            line_height_opt: self.line_height_opt,
            font_id: self.font_id,
            glyph_id: self.glyph_id,
            x: self.x,
            y: self.y,
            w: self.w,
            level: self.level,
            x_offset: self.x_offset,
            y_offset: self.y_offset,
            color_opt: self.color_opt,
            metadata: self.metadata,
            cache_key_flags: self.cache_key_flags,
        };

        layout_glyph.physical(offset, scale)
    }

    /// Get glyph flags (converted from cache key flags)
    pub fn glyph_flags(&self) -> crate::types::GlyphFlags {
        use cosmyc_text::CacheKeyFlags;

        use crate::types::GlyphFlags;

        let mut flags = GlyphFlags::empty();

        // Convert cosmyc-text cache key flags to our glyph flags
        // Cache key flags affect rendering, while glyph flags affect text processing

        if self.cache_key_flags.contains(CacheKeyFlags::FAKE_ITALIC) {
            // Fake italic may affect character boundaries
            flags |= GlyphFlags::UNSAFE_TO_BREAK;
        }

        if self.cache_key_flags.bits() & 2 != 0 {
            // Disabled hinting may affect glyph positioning
            flags |= GlyphFlags::UNSAFE_TO_CONCAT;
        }

        if self.cache_key_flags.bits() & 4 != 0 {
            // Pixel fonts may have specific positioning requirements
            flags |= GlyphFlags::COMPONENT_GLYPH;
        }

        // Mark as cluster start if this is the beginning of a text cluster
        if self.start != self.end {
            flags |= GlyphFlags::IS_CLUSTER_START;
        }

        flags
    }
}
