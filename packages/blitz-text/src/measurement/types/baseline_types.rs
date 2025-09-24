//! Baseline and font metrics type definitions
//!
//! This module contains CSS baseline alignment types and font metrics structures
//! used throughout the text measurement system.

/// CSS baseline alignment types as defined in the CSS specification
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum CSSBaseline {
    /// Alphabetic baseline (default for most scripts)
    Alphabetic,
    /// Ideographic baseline (for CJK scripts)
    Ideographic,
    /// Hanging baseline (for Devanagari and related scripts)
    Hanging,
    /// Mathematical baseline (for mathematical formulas)
    Mathematical,
    /// Central baseline (middle of font)
    Central,
    /// Middle baseline (x-height center)
    Middle,
    /// Text-top baseline (top of text content area)
    TextTop,
    /// Text-bottom baseline (bottom of text content area)
    TextBottom,
}

impl Default for CSSBaseline {
    fn default() -> Self {
        CSSBaseline::Alphabetic
    }
}

/// Font metrics for baseline calculations
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FontMetrics {
    pub units_per_em: u16,
    pub ascent: i16,
    pub descent: i16,
    pub line_gap: i16,
    pub x_height: Option<i16>,
    pub cap_height: Option<i16>,
    pub ideographic_baseline: Option<i16>,
    pub hanging_baseline: Option<i16>,
    pub mathematical_baseline: Option<i16>,
    pub average_char_width: f32,
    pub max_char_width: f32,
    pub underline_position: f32,
    pub underline_thickness: f32,
    pub strikethrough_position: f32,
    pub strikethrough_thickness: f32,
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 90,
            x_height: Some(500),
            cap_height: Some(700),
            ideographic_baseline: Some(-120),
            hanging_baseline: Some(800),
            mathematical_baseline: Some(350),
            average_char_width: 500.0,
            max_char_width: 1000.0,
            underline_position: -100.0,
            underline_thickness: 50.0,
            strikethrough_position: 300.0,
            strikethrough_thickness: 50.0,
        }
    }
}
