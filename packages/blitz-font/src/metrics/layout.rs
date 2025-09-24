//! Font layout metrics for precise text positioning and scaling
//!
//! This module provides zero-allocation layout calculations optimized for
//! high-performance text rendering in the Blitz browser engine.

use super::core::FontMetrics;

/// Font layout metrics for precise text positioning
#[derive(Debug, Clone)]
pub struct FontLayoutMetrics {
    pub font_metrics: FontMetrics,
    pub font_size: f32,
    pub line_height_multiplier: f32,
}

impl FontLayoutMetrics {
    /// Create layout metrics from font metrics and size
    #[inline]
    pub const fn new(font_metrics: FontMetrics, font_size: f32) -> Self {
        Self {
            font_metrics,
            font_size,
            line_height_multiplier: 1.2,
        }
    }

    /// Set custom line height multiplier (builder pattern)
    #[inline]
    pub const fn with_line_height_multiplier(mut self, multiplier: f32) -> Self {
        self.line_height_multiplier = multiplier;
        self
    }

    /// Get scaled ascent for the given font size
    #[inline]
    pub fn scaled_ascent(&self) -> f32 {
        self.font_metrics.ascent * self.font_size
    }

    /// Get scaled descent for the given font size
    #[inline]
    pub fn scaled_descent(&self) -> f32 {
        self.font_metrics.descent * self.font_size
    }

    /// Get scaled line gap for the given font size
    #[inline]
    pub fn scaled_line_gap(&self) -> f32 {
        self.font_metrics.line_gap * self.font_size
    }

    /// Get effective line height for layout
    #[inline]
    pub fn layout_line_height(&self) -> f32 {
        (self.font_metrics.line_height() * self.font_size) * self.line_height_multiplier
    }

    /// Get baseline offset from top of line
    #[inline]
    pub fn baseline_offset(&self) -> f32 {
        self.scaled_ascent()
    }

    /// Get text bounding box height
    #[inline]
    pub fn text_bounding_height(&self) -> f32 {
        self.scaled_ascent() + self.scaled_descent().abs()
    }

    /// Calculate position for underline
    #[inline]
    pub fn underline_position(&self) -> f32 {
        let (pos, _) = self.font_metrics.underline_metrics();
        pos * self.font_size
    }

    /// Calculate thickness for underline
    #[inline]
    pub fn underline_thickness(&self) -> f32 {
        let (_, thickness) = self.font_metrics.underline_metrics();
        thickness * self.font_size
    }

    /// Calculate position for strikeout
    #[inline]
    pub fn strikeout_position(&self) -> f32 {
        let (pos, _) = self.font_metrics.strikeout_metrics();
        pos * self.font_size
    }

    /// Calculate thickness for strikeout
    #[inline]
    pub fn strikeout_thickness(&self) -> f32 {
        let (_, thickness) = self.font_metrics.strikeout_metrics();
        thickness * self.font_size
    }
}
