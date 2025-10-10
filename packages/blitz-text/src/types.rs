//! Core data structures for text shaping

use cosmyc_text::{Attrs, Family};
use unicode_bidi::Level;
use unicode_script::Script;

use crate::features::FeatureSettings;

/// Unique key for caching shaped text results
#[derive(Debug, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct ShapingCacheKey {
    pub text_hash: u64,
    pub attrs_hash: u64,
    pub max_width_hash: u64,
    pub feature_hash: u64,
}

/// Complete shaped text with all runs and metrics
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText {
    pub runs: Vec<ShapedRun>,
    pub total_width: f32,
    pub total_height: f32,
    pub baseline: f32,
    pub line_count: usize,
    pub shaped_at: std::time::Instant,
    pub cache_key: ShapingCacheKey,
}

/// Single shaped run with consistent script and direction
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRun {
    pub glyphs: Vec<ShapedGlyph>,
    pub script: Script,
    pub direction: TextDirection,
    pub language: Option<&'static str>,
    pub level: Level,
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub start_index: usize,
    pub end_index: usize,
}

/// Shaped glyph with positioning and metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    pub glyph_id: u16,
    pub cluster: u32,
    pub x_advance: f32,
    pub y_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub flags: GlyphFlags,
    pub font_size: f32,
    pub color: Option<u32>,
}

/// Text direction for proper rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

bitflags::bitflags! {
    /// Glyph flags for advanced text processing
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GlyphFlags: u32 {
        const UNSAFE_TO_BREAK = 0x00000001;
        const UNSAFE_TO_CONCAT = 0x00000002;
        const SAFE_TO_INSERT_TATWEEL = 0x00000004;
        const IS_CLUSTER_START = 0x00000008;
        const CONTINUATION_CLUSTER = 0x00000010;
        const CURSIVE_CONNECTION = 0x00000020;
        const MARKS_ATTACHED = 0x00000040;
        const COMPONENT_GLYPH = 0x00000080;
    }
}

/// Text analysis results for processing optimization
#[derive(Debug, Clone)]
pub struct TextAnalysis {
    pub script_runs: Vec<ScriptRun>,
    pub base_direction: TextDirection,
    pub has_complex_scripts: bool,
    pub requires_bidi: bool,
    pub complexity_score: u32,
}

/// Script run with boundaries and metadata
#[derive(Debug, Clone)]
pub struct ScriptRun {
    pub start: usize,
    pub end: usize,
    pub script: Script,
    pub complexity: ScriptComplexity,
}

/// Text run ready for shaping
#[derive(Debug, Clone)]
pub struct TextRun {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub script: Script,
    pub direction: TextDirection,
    pub level: Level,
    pub attrs: cosmyc_text::AttrsOwned,
    pub language: Option<&'static str>,
    pub features: &'static FeatureSettings,
}

/// Script complexity classification for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptComplexity {
    Simple,      // Latin, Cyrillic, Greek
    Moderate,    // Hebrew, Thai
    Complex,     // Arabic, Devanagari, Myanmar
    VeryComplex, // Khmer, Tibetan
}

/// Line breaking information for text layout
#[derive(Debug, Clone)]
pub struct LineBreak {
    pub position: usize,
    pub mandatory: bool,
    pub width_before: f32,
    pub width_after: f32,
}

/// Font key for glyph identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontKey {
    pub family_id: u32,
    pub weight: u16,
    pub style: u8,
    pub stretch: u8,
}

impl Default for FontKey {
    fn default() -> Self {
        Self {
            family_id: 0,
            weight: 400,
            style: 0,
            stretch: 100,
        }
    }
}

impl FontKey {
    /// Create font key from cosmyc-text attributes
    #[inline]
    pub fn from_attrs(attrs: &Attrs) -> Self {
        Self {
            family_id: match attrs.family {
                Family::Name(name) => name.len() as u32,
                Family::SansSerif => "sans-serif".len() as u32,
                Family::Serif => "serif".len() as u32,
                Family::Cursive => "cursive".len() as u32,
                Family::Fantasy => "fantasy".len() as u32,
                Family::Monospace => "monospace".len() as u32,
            },
            weight: attrs.weight.0, // Weight is Weight(u16)
            style: attrs.style as u8,
            stretch: attrs.stretch.to_number() as u8,
        }
    }

    /// Create font key with specific parameters
    #[inline]
    pub fn new(family_id: u32, weight: u16, style: u8, stretch: u8) -> Self {
        Self {
            family_id,
            weight,
            style,
            stretch,
        }
    }
}

impl TextDirection {
    #[inline]
    pub fn is_horizontal(self) -> bool {
        matches!(
            self,
            TextDirection::LeftToRight | TextDirection::RightToLeft
        )
    }

    #[inline]
    pub fn is_rtl(self) -> bool {
        matches!(
            self,
            TextDirection::RightToLeft | TextDirection::BottomToTop
        )
    }
}

impl ScriptComplexity {
    pub fn from_script(script: Script) -> Self {
        match script {
            Script::Latin | Script::Cyrillic | Script::Greek | Script::Armenian => {
                ScriptComplexity::Simple
            }
            Script::Hebrew | Script::Thai | Script::Lao => ScriptComplexity::Moderate,
            Script::Arabic
            | Script::Devanagari
            | Script::Bengali
            | Script::Gurmukhi
            | Script::Gujarati
            | Script::Oriya
            | Script::Tamil
            | Script::Telugu
            | Script::Kannada
            | Script::Malayalam
            | Script::Sinhala
            | Script::Myanmar => ScriptComplexity::Complex,
            Script::Khmer | Script::Tibetan | Script::Mongolian => ScriptComplexity::VeryComplex,
            _ => ScriptComplexity::Simple,
        }
    }

    pub fn cache_priority(self) -> u32 {
        match self {
            ScriptComplexity::Simple => 1,
            ScriptComplexity::Moderate => 2,
            ScriptComplexity::Complex => 3,
            ScriptComplexity::VeryComplex => 4,
        }
    }
}

/// Text metrics for layout calculations
#[derive(Debug, Clone)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub baseline: f32,
}

impl Default for TextMetrics {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            ascent: 0.0,
            descent: 0.0,
            line_gap: 0.0,
            baseline: 0.0,
        }
    }
}

/// Bidi run information
#[derive(Debug, Clone)]
pub struct BidiRun {
    pub start: usize,
    pub end: usize,
    pub level: Level,
    pub direction: TextDirection,
}

impl BidiRun {
    pub fn new(start: usize, end: usize, level: Level) -> Self {
        let direction = if level.is_rtl() {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        };

        Self {
            start,
            end,
            level,
            direction,
        }
    }
}

/// Shaping context for advanced features
#[derive(Debug, Clone)]
pub struct ShapingContext {
    pub language: Option<&'static str>,
    pub script: Script,
    pub direction: TextDirection,
    pub features: &'static FeatureSettings,
    pub font_size: f32,
}

impl Default for ShapingContext {
    fn default() -> Self {
        Self {
            language: None,
            script: Script::Latin,
            direction: TextDirection::LeftToRight,
            features: &crate::features::DEFAULT_FEATURES,
            font_size: 16.0,
        }
    }
}
