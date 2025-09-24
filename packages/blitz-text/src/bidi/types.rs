//! BiDi types and data structures
//!
//! This module contains all the core types, enums, and data structures
//! used throughout the bidirectional text processing system.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use unicode_script::Script;

/// Serializable wrapper for Unicode Script
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum SerializableScript {
    #[default]
    Unknown,
    Latin,
    Greek,
    Cyrillic,
    Arabic,
    Hebrew,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Thai,
    Lao,
    Tibetan,
    Myanmar,
    Georgian,
    Hangul,
    Ethiopic,
    Cherokee,
    // Common represents the default for punctuation, numbers, etc.
    Common,
    // Inherited represents characters that inherit script from preceding characters
    Inherited,
    // Other represents any script not explicitly enumerated above
    Other(String),
}

impl From<Script> for SerializableScript {
    fn from(script: Script) -> Self {
        match script {
            Script::Latin => SerializableScript::Latin,
            Script::Greek => SerializableScript::Greek,
            Script::Cyrillic => SerializableScript::Cyrillic,
            Script::Arabic => SerializableScript::Arabic,
            Script::Hebrew => SerializableScript::Hebrew,
            Script::Devanagari => SerializableScript::Devanagari,
            Script::Bengali => SerializableScript::Bengali,
            Script::Gurmukhi => SerializableScript::Gurmukhi,
            Script::Gujarati => SerializableScript::Gujarati,
            Script::Oriya => SerializableScript::Oriya,
            Script::Tamil => SerializableScript::Tamil,
            Script::Telugu => SerializableScript::Telugu,
            Script::Kannada => SerializableScript::Kannada,
            Script::Malayalam => SerializableScript::Malayalam,
            Script::Thai => SerializableScript::Thai,
            Script::Lao => SerializableScript::Lao,
            Script::Tibetan => SerializableScript::Tibetan,
            Script::Myanmar => SerializableScript::Myanmar,
            Script::Georgian => SerializableScript::Georgian,
            Script::Hangul => SerializableScript::Hangul,
            Script::Ethiopic => SerializableScript::Ethiopic,
            Script::Cherokee => SerializableScript::Cherokee,
            Script::Common => SerializableScript::Common,
            Script::Inherited => SerializableScript::Inherited,
            _ => SerializableScript::Other(format!("{:?}", script)),
        }
    }
}

impl From<SerializableScript> for Script {
    fn from(script: SerializableScript) -> Self {
        match script {
            SerializableScript::Latin => Script::Latin,
            SerializableScript::Greek => Script::Greek,
            SerializableScript::Cyrillic => Script::Cyrillic,
            SerializableScript::Arabic => Script::Arabic,
            SerializableScript::Hebrew => Script::Hebrew,
            SerializableScript::Devanagari => Script::Devanagari,
            SerializableScript::Bengali => Script::Bengali,
            SerializableScript::Gurmukhi => Script::Gurmukhi,
            SerializableScript::Gujarati => Script::Gujarati,
            SerializableScript::Oriya => Script::Oriya,
            SerializableScript::Tamil => Script::Tamil,
            SerializableScript::Telugu => Script::Telugu,
            SerializableScript::Kannada => Script::Kannada,
            SerializableScript::Malayalam => Script::Malayalam,
            SerializableScript::Thai => Script::Thai,
            SerializableScript::Lao => Script::Lao,
            SerializableScript::Tibetan => Script::Tibetan,
            SerializableScript::Myanmar => Script::Myanmar,
            SerializableScript::Georgian => Script::Georgian,
            SerializableScript::Hangul => Script::Hangul,
            SerializableScript::Ethiopic => Script::Ethiopic,
            SerializableScript::Cherokee => Script::Cherokee,
            SerializableScript::Common => Script::Common,
            SerializableScript::Inherited => Script::Inherited,
            SerializableScript::Unknown | SerializableScript::Other(_) => Script::Unknown,
        }
    }
}

impl SerializableScript {
    /// Convert from unicode_script::Script to SerializableScript
    pub fn from_script(script: Script) -> Self {
        Self::from(script)
    }
}

use crate::shaping::types::{ScriptComplexity, TextDirection};

/// Text direction for bidi text processing
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    Default,
)]
pub enum Direction {
    #[default]
    LeftToRight,
    RightToLeft,
    Auto,
}

/// BiDi rendering configuration options
#[derive(Debug, Clone)]
pub struct BidiRenderOptions {
    pub base_direction: Direction,
    pub text_orientation: TextOrientation,
    pub writing_mode: WritingMode,
    pub unicode_bidi: UnicodeBidi,
}

/// Text orientation for vertical writing modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOrientation {
    Mixed,
    Upright,
    Sideways,
}

/// Writing mode specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritingMode {
    HorizontalTopBottom,
    VerticalRightLeft,
    VerticalLeftRight,
}

/// Unicode BiDi property values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeBidi {
    Normal,
    Embed,
    Isolate,
    BidiOverride,
    IsolateOverride,
    Plaintext,
}

/// Cursor position in bidirectional text
#[derive(Debug, Clone)]
pub struct CursorPosition {
    pub logical_index: usize,
    pub visual_x: f32,
    pub line_index: usize,
    pub is_trailing: bool,
    pub direction: Direction,
    pub level: u8,
}

/// Text selection rectangle in BiDi text
#[derive(Debug, Clone)]
pub struct SelectionRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub direction: Direction,
}

/// Text selection for BiDi text
#[derive(Debug, Clone)]
pub struct Selection {
    pub logical_start: usize,
    pub logical_end: usize,
    pub visual_start: CursorPosition,
    pub visual_end: CursorPosition,
    pub is_empty: bool,
}

/// BiDi rendering statistics
#[derive(Debug, Default)]
pub struct BidiStats {
    pub total_processed: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_processing_time_ns: u64,
}

/// Cache key for BiDi processing results
#[derive(Debug, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct BidiCacheKey {
    pub text_hash: u64,
    pub base_direction: Direction,
}

/// Cache key for cursor positions
#[derive(Debug, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct CursorCacheKey {
    pub text_hash: u64,
    pub logical_index: usize,
    pub base_direction: Direction,
}

/// Processed bidirectional text with visual ordering
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ProcessedBidi {
    pub text: String,
    pub visual_runs: Vec<VisualRun>,
    pub logical_to_visual: Vec<usize>,
    pub visual_to_logical: Vec<usize>,
    pub base_direction: Direction,
    pub paragraph_level: u8,
}

/// Visual run with consistent direction and script
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct VisualRun {
    pub text: String,
    pub start_index: usize,
    pub end_index: usize,
    pub direction: Direction,
    pub level: u8,
    pub script: SerializableScript,
    pub complexity: ScriptComplexity,
    pub visual_order: usize,
}

/// Multi-line bidirectional text processing result
#[derive(Debug, Clone)]
pub struct MultiLineBidiResult {
    pub paragraphs: Vec<ParagraphBidi>,
    pub total_lines: usize,
    pub base_direction: Direction,
}

/// Paragraph-level bidirectional processing result
#[derive(Debug, Clone)]
pub struct ParagraphBidi {
    pub paragraph_index: usize,
    pub lines: Vec<LineBidi>,
    pub base_direction: Direction,
}

/// Line-level bidirectional processing result
#[derive(Debug, Clone)]
pub struct LineBidi {
    pub line_index: usize,
    pub processed_bidi: ProcessedBidi,
    pub line_height: f32,
    pub baseline_offset: f32,
    pub visual_width: f32,
    pub break_opportunity: bool,
}

/// Line break information for BiDi text
#[derive(Debug, Clone)]
pub struct LineBreakInfo {
    pub text: String,
    pub break_positions: Vec<usize>,
    pub break_opportunities: Vec<bool>,
    pub line_widths: Vec<f32>,
    pub max_width: f32,
}

/// Text selection in bidirectional text
#[derive(Debug, Clone)]
pub struct BidiSelection {
    pub position: usize,
    pub length: usize,
    pub rectangles: Vec<SelectionRect>,
}

/// Line metrics for BiDi rendering
#[derive(Debug, Clone)]
pub struct LineMetrics {
    pub line_height: f32,
    pub baseline_offset: f32,
    pub ascent: f32,
    pub descent: f32,
}

/// BiDi processing error types
#[derive(Debug, thiserror::Error)]
pub enum BidiError {
    #[error("BiDi text processing failed: {0}")]
    ProcessingFailed(String),

    #[error("Invalid text direction: {0}")]
    InvalidDirection(String),

    #[error("Cache operation failed: {0}")]
    CacheError(String),

    #[error("Rendering failed: {0}")]
    RenderingFailed(String),

    #[error("Invalid cursor position: {position} in text of length {text_length}")]
    InvalidCursorPosition { position: usize, text_length: usize },

    #[error("Line breaking failed: {0}")]
    LineBreakingFailed(String),

    #[error("Script analysis failed: {0}")]
    ScriptAnalysisFailed(String),

    #[error("Statistical calculation failed: {0}")]
    StatisticalCalculationFailed(String),

    #[error(
        "Insufficient data for statistical analysis: expected at least {expected}, got {actual}"
    )]
    InsufficientStatisticalData { expected: usize, actual: usize },
}

impl Default for BidiRenderOptions {
    fn default() -> Self {
        Self {
            base_direction: Direction::Auto,
            text_orientation: TextOrientation::Mixed,
            writing_mode: WritingMode::HorizontalTopBottom,
            unicode_bidi: UnicodeBidi::Normal,
        }
    }
}

impl BidiCacheKey {
    /// Create new cache key from text and direction
    pub fn new(text: &str, base_direction: Direction) -> Self {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        Self {
            text_hash: hasher.finish(),
            base_direction,
        }
    }
}

impl CursorCacheKey {
    /// Create new cursor cache key
    pub fn new(text: &str, logical_index: usize, base_direction: Direction) -> Self {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        Self {
            text_hash: hasher.finish(),
            logical_index,
            base_direction,
        }
    }
}

// Hash implementations for configuration enums
impl Hash for TextOrientation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl Hash for WritingMode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl Hash for UnicodeBidi {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl From<TextDirection> for Direction {
    fn from(direction: TextDirection) -> Self {
        match direction {
            TextDirection::LeftToRight => Direction::LeftToRight,
            TextDirection::RightToLeft => Direction::RightToLeft,
            TextDirection::TopToBottom => Direction::LeftToRight, // Map vertical to horizontal
            TextDirection::BottomToTop => Direction::RightToLeft, // Map vertical to RTL
        }
    }
}

impl From<Direction> for TextDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::LeftToRight => TextDirection::LeftToRight,
            Direction::RightToLeft => TextDirection::RightToLeft,
            Direction::Auto => TextDirection::LeftToRight, // Default fallback
        }
    }
}
