//! Enhanced measurement data types and conversions

use cosmyc_text::{Align, PhysicalGlyph, Shaping, Wrap};

use crate::measurement::types::{
    CharacterPosition, FontMetrics, LineMeasurement, TextBounds, TextMeasurement,
};

/// Enhanced text measurement result with comprehensive cosmyc-text data
#[derive(Debug, Clone)]
pub struct EnhancedTextMeasurement {
    // Basic measurements (from original TextMeasurement)
    pub content_width: f32,
    pub content_height: f32,
    pub line_height: f32,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub x_height: f32,
    pub cap_height: f32,
    pub advance_width: f32,
    pub bounds: TextBounds,
    pub line_measurements: Vec<LineMeasurement>,
    pub total_character_count: usize,
    pub baseline_offset: f32,
    pub measured_at: std::time::Instant,

    // Enhanced measurements using cosmyc-text APIs
    pub font_metrics: FontMetrics,
    pub baseline_info: BaselineInfo,
    pub physical_glyphs: Vec<PhysicalGlyph>,
    pub all_character_positions: Vec<CharacterPosition>,
    pub text_wrap: Wrap,
    pub text_align: Align,
    pub text_shaping: Shaping,
    pub glyph_count: usize,
}

/// Comprehensive baseline information for all CSS baseline types
#[derive(Debug, Clone)]
pub struct BaselineInfo {
    pub alphabetic: f32,
    pub ideographic: f32,
    pub hanging: f32,
    pub mathematical: f32,
    pub central: f32,
    pub middle: f32,
    pub text_top: f32,
    pub text_bottom: f32,
}

impl From<EnhancedTextMeasurement> for TextMeasurement {
    fn from(enhanced: EnhancedTextMeasurement) -> Self {
        Self {
            content_width: enhanced.content_width,
            content_height: enhanced.content_height,
            line_height: enhanced.line_height,
            baseline: enhanced.baseline,
            ascent: enhanced.ascent,
            descent: enhanced.descent,
            line_gap: enhanced.line_gap,
            x_height: enhanced.x_height,
            cap_height: enhanced.cap_height,
            advance_width: enhanced.advance_width,
            bounds: enhanced.bounds,
            line_measurements: enhanced.line_measurements,
            total_character_count: enhanced.total_character_count,
            baseline_offset: enhanced.baseline_offset,
            measured_at: enhanced.measured_at,
        }
    }
}

impl From<TextMeasurement> for EnhancedTextMeasurement {
    fn from(basic: TextMeasurement) -> Self {
        Self {
            content_width: basic.content_width,
            content_height: basic.content_height,
            line_height: basic.line_height,
            baseline: basic.baseline,
            ascent: basic.ascent,
            descent: basic.descent,
            line_gap: basic.line_gap,
            x_height: basic.x_height,
            cap_height: basic.cap_height,
            advance_width: basic.advance_width,
            bounds: basic.bounds,
            line_measurements: basic.line_measurements,
            total_character_count: basic.total_character_count,
            baseline_offset: basic.baseline_offset,
            measured_at: basic.measured_at,

            // Default values for enhanced fields
            font_metrics: FontMetrics::default(),
            baseline_info: BaselineInfo {
                alphabetic: basic.baseline,
                ideographic: basic.baseline,
                hanging: basic.baseline - basic.ascent * 0.8,
                mathematical: basic.baseline - basic.ascent * 0.5,
                central: basic.baseline - basic.ascent * 0.5,
                middle: basic.baseline - basic.x_height * 0.5,
                text_top: basic.baseline - basic.ascent,
                text_bottom: basic.baseline + basic.descent,
            },
            physical_glyphs: Vec::new(),
            all_character_positions: Vec::new(),
            text_wrap: Wrap::Word,
            text_align: Align::Left,
            text_shaping: Shaping::Basic,
            glyph_count: 0,
        }
    }
}
