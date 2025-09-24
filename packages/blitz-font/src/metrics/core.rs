//! Core font metrics extraction and parsing functionality
//!
//! This module provides zero-allocation font metrics extraction from font files
//! optimized for high-performance text rendering in the Blitz browser engine.

/// Core font metrics extracted from font files
#[derive(Debug, Clone)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub units_per_em: u16,
    pub x_height: Option<f32>,
    pub cap_height: Option<f32>,
    pub average_width: Option<f32>,
    pub max_width: Option<f32>,
    pub underline_position: Option<f32>,
    pub underline_thickness: Option<f32>,
    pub strikeout_position: Option<f32>,
    pub strikeout_thickness: Option<f32>,
}

impl FontMetrics {
    /// Create font metrics from ttf-parser Face
    #[inline]
    pub fn from_face(face: &ttf_parser::Face) -> Self {
        let units_per_em = face.units_per_em();
        let scale_factor = 1.0 / units_per_em as f32;

        // Calculate width metrics from glyph advances
        let (average_width, max_width) =
            Self::calculate_width_metrics_optimized(face, scale_factor);

        Self {
            ascent: face.ascender() as f32 * scale_factor,
            descent: face.descender() as f32 * scale_factor,
            line_gap: face.line_gap() as f32 * scale_factor,
            units_per_em,
            x_height: face.x_height().map(|h| h as f32 * scale_factor),
            cap_height: face.capital_height().map(|h| h as f32 * scale_factor),
            average_width,
            max_width,
            underline_position: face
                .underline_metrics()
                .map(|m| m.position as f32 * scale_factor),
            underline_thickness: face
                .underline_metrics()
                .map(|m| m.thickness as f32 * scale_factor),
            strikeout_position: face
                .strikeout_metrics()
                .map(|m| m.position as f32 * scale_factor),
            strikeout_thickness: face
                .strikeout_metrics()
                .map(|m| m.thickness as f32 * scale_factor),
        }
    }

    /// Calculate average and maximum character width from glyph advances (optimized for zero-allocation)
    #[inline]
    fn calculate_width_metrics_optimized(
        face: &ttf_parser::Face,
        scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        let mut total_advance = 0.0f32;
        let mut max_advance = 0.0f32;
        let mut count = 0u32;

        // Iterate through ASCII printable characters (space to tilde) - zero allocation
        for byte in 32u8..=126 {
            let c = byte as char;
            if let Some(glyph_id) = face.glyph_index(c) {
                if let Some(advance) = face.glyph_hor_advance(glyph_id) {
                    let normalized_advance = advance as f32 * scale_factor;
                    total_advance += normalized_advance;
                    max_advance = max_advance.max(normalized_advance);
                    count += 1;
                }
            }
        }

        let average_width = if count > 0 {
            Some(total_advance / count as f32)
        } else {
            None
        };

        let max_width = if count > 0 { Some(max_advance) } else { None };

        (average_width, max_width)
    }

    /// Create default metrics for fallback cases
    #[inline]
    pub const fn default_metrics() -> Self {
        Self {
            ascent: 0.8,
            descent: -0.2,
            line_gap: 0.0,
            units_per_em: 1000,
            x_height: Some(0.5),
            cap_height: Some(0.7),
            average_width: Some(0.5),
            max_width: Some(1.0),
            underline_position: Some(-0.1),
            underline_thickness: Some(0.05),
            strikeout_position: Some(0.25),
            strikeout_thickness: Some(0.05),
        }
    }

    /// Get the total line height (ascent + descent + line_gap)
    #[inline]
    pub fn line_height(&self) -> f32 {
        self.ascent + self.descent.abs() + self.line_gap
    }

    /// Get the baseline-to-baseline distance
    #[inline]
    pub fn baseline_to_baseline(&self) -> f32 {
        self.ascent + self.descent.abs() + self.line_gap
    }

    /// Get the text height (ascent + descent, without line gap)
    #[inline]
    pub fn text_height(&self) -> f32 {
        self.ascent + self.descent.abs()
    }

    /// Get the cap height, falling back to ascent if not available
    #[inline]
    pub fn effective_cap_height(&self) -> f32 {
        self.cap_height.unwrap_or(self.ascent * 0.7)
    }

    /// Get the x-height, falling back to half the cap height if not available
    #[inline]
    pub fn effective_x_height(&self) -> f32 {
        self.x_height.unwrap_or(self.effective_cap_height() * 0.5)
    }

    /// Get underline position and thickness
    #[inline]
    pub fn underline_metrics(&self) -> (f32, f32) {
        (
            self.underline_position.unwrap_or(self.descent * 0.5),
            self.underline_thickness
                .unwrap_or(self.text_height() * 0.05),
        )
    }

    /// Get strikeout position and thickness
    #[inline]
    pub fn strikeout_metrics(&self) -> (f32, f32) {
        (
            self.strikeout_position
                .unwrap_or(self.effective_x_height() * 0.5),
            self.strikeout_thickness
                .unwrap_or(self.text_height() * 0.05),
        )
    }

    /// Scale metrics by a given factor (zero allocation)
    #[inline]
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            ascent: self.ascent * factor,
            descent: self.descent * factor,
            line_gap: self.line_gap * factor,
            units_per_em: self.units_per_em,
            x_height: self.x_height.map(|v| v * factor),
            cap_height: self.cap_height.map(|v| v * factor),
            average_width: self.average_width.map(|v| v * factor),
            max_width: self.max_width.map(|v| v * factor),
            underline_position: self.underline_position.map(|v| v * factor),
            underline_thickness: self.underline_thickness.map(|v| v * factor),
            strikeout_position: self.strikeout_position.map(|v| v * factor),
            strikeout_thickness: self.strikeout_thickness.map(|v| v * factor),
        }
    }

    /// Get the bounding box height for text with this font
    #[inline]
    pub fn bounding_box_height(&self) -> f32 {
        self.line_height()
    }

    /// Get the recommended line spacing for multi-line text
    #[inline]
    pub fn recommended_line_spacing(&self) -> f32 {
        self.line_height() * 1.2
    }

    /// Check if the font metrics appear valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.units_per_em > 0 && self.ascent > 0.0 && self.descent < 0.0 && self.line_gap >= 0.0
    }

    /// Get a quality score for these metrics (0.0 = poor, 1.0 = excellent)
    /// Optimized to avoid branching where possible
    #[inline]
    pub fn quality_score(&self) -> f32 {
        let mut score = 0.0;

        // Check if basic metrics are present
        if self.ascent > 0.0 && self.descent < 0.0 {
            score += 0.3;
        }

        // Check if x_height is available
        score += if self.x_height.is_some() { 0.2 } else { 0.0 };

        // Check if cap_height is available
        score += if self.cap_height.is_some() { 0.2 } else { 0.0 };

        // Check if width metrics are available
        score += if self.average_width.is_some() && self.max_width.is_some() {
            0.15
        } else {
            0.0
        };

        // Check if underline metrics are available
        score += if self.underline_position.is_some() && self.underline_thickness.is_some() {
            0.075
        } else {
            0.0
        };

        // Check if strikeout metrics are available
        score += if self.strikeout_position.is_some() && self.strikeout_thickness.is_some() {
            0.075
        } else {
            0.0
        };

        score
    }
}

impl Default for FontMetrics {
    #[inline]
    fn default() -> Self {
        Self::default_metrics()
    }
}
