//! Core data structures for text shaping
//!
//! This module contains all the fundamental types used throughout the text shaping
//! pipeline, including shaped text results, glyph data, and metadata structures.

use cosmyc_text::Attrs;
use goldylox::cache::traits::metadata::CacheValueMetadata;
use goldylox::cache::traits::supporting_types::HashAlgorithm;
use goldylox::cache::traits::supporting_types::{HashContext, Priority, SizeEstimator};
use goldylox::cache::traits::types_and_enums::{CompressionHint, PriorityClass};
use goldylox::traits::CacheValue;
use serde::{Deserialize, Serialize};
use unicode_bidi::Level;
use unicode_script::Script;

/// Unique key for caching shaped text results
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct ShapingCacheKey {
    pub text_hash: u64,
    pub attrs_hash: u64,
    pub max_width_hash: u64,
    pub feature_hash: u64,
}

/// Complete shaped text with all runs and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapedText {
    pub runs: Vec<ShapedRun>,
    pub total_width: f32,
    pub total_height: f32,
    pub baseline: f32,
    pub line_count: usize,
    #[serde(skip, default = "std::time::Instant::now")]
    pub shaped_at: std::time::Instant,
    pub cache_key: ShapingCacheKey,
}

impl Default for ShapedText {
    fn default() -> Self {
        Self {
            runs: Vec::new(),
            total_width: 0.0,
            total_height: 0.0,
            baseline: 0.0,
            line_count: 0,
            shaped_at: std::time::Instant::now(),
            cache_key: ShapingCacheKey::default(),
        }
    }
}

/// Single shaped run with consistent script and direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapedRun {
    pub glyphs: Vec<ShapedGlyph>,
    #[serde(with = "script_serde")]
    pub script: Script,
    pub direction: TextDirection,
    pub language: Option<String>,
    #[serde(with = "level_serde")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

bitflags::bitflags! {
    /// Glyph flags for advanced text processing
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRun {
    pub start: usize,
    pub end: usize,
    #[serde(with = "script_serde")]
    pub script: Script,
    pub complexity: ScriptComplexity,
}

/// Text run ready for shaping
#[derive(Debug, Clone)]
pub struct TextRun<'a> {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub script: Script,
    pub direction: TextDirection,
    pub level: Level,
    pub attrs: Attrs<'a>,
    pub language: Option<String>,
    pub features: &'static FeatureSettings,
}

/// Script complexity classification for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScriptComplexity {
    #[default]
    Simple, // Latin, Cyrillic, Greek
    Moderate,    // Hebrew, Thai
    Complex,     // Arabic, Devanagari, Myanmar
    VeryComplex, // Khmer, Tibetan
}

/// OpenType feature settings for different scripts
#[derive(Debug, Clone)]
pub struct FeatureSettings {
    pub ligatures: bool,
    pub kerning: bool,
    pub contextual_alternates: bool,
    pub stylistic_sets: &'static [u8],
    pub opentype_features: &'static [(&'static str, u32)],
}

impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            ligatures: true,
            kerning: true,
            contextual_alternates: true,
            stylistic_sets: &[],
            opentype_features: &[],
        }
    }
}

/// Supporting types for Goldylox cache integration

/// Hash context for text shaping cache keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextShapingHashContext {
    pub seed: u64,
    pub algorithm: &'static str,
}

impl Default for TextShapingHashContext {
    fn default() -> Self {
        Self {
            seed: 0x9e3779b97f4a7c15, // Golden ratio constant
            algorithm: "ahash",
        }
    }
}

impl TextShapingHashContext {
    pub fn new() -> Self {
        Self::default()
    }
}

impl HashContext for TextShapingHashContext {
    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::AHash
    }

    fn seed(&self) -> u64 {
        self.seed
    }

    fn compute_hash<T: std::hash::Hash>(&self, value: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        value.hash(&mut hasher);
        hasher.finish()
    }
}

/// Priority for text shaping cache entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TextShapingPriority {
    pub complexity_score: u32,
    pub access_frequency: u32,
}

impl Default for TextShapingPriority {
    fn default() -> Self {
        Self {
            complexity_score: 1,
            access_frequency: 1,
        }
    }
}

impl Priority for TextShapingPriority {
    fn value(&self) -> u32 {
        (self.complexity_score) * 100 + (self.access_frequency)
    }

    fn class(&self) -> PriorityClass {
        if self.complexity_score > 2 {
            PriorityClass::High
        } else {
            PriorityClass::Low
        }
    }

    fn adjust(&self, factor: f32) -> Self {
        Self {
            complexity_score: self.complexity_score,
            access_frequency: (self.access_frequency as f32 * factor) as u32,
        }
    }
}

/// Size estimator for text shaping data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextShapingSizeEstimator {
    pub base_size: usize,
    pub glyph_multiplier: f32,
}

impl Default for TextShapingSizeEstimator {
    fn default() -> Self {
        Self {
            base_size: std::mem::size_of::<ShapedText>(),
            glyph_multiplier: 1.5, // Account for run overhead
        }
    }
}

impl TextShapingSizeEstimator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SizeEstimator for TextShapingSizeEstimator {
    fn estimate_size<T>(&self, _value: &T) -> usize {
        self.base_size
    }

    fn deep_estimate_size<T>(&self, _value: &T) -> usize {
        let base = std::mem::size_of::<T>();
        (base as f32 * self.glyph_multiplier) as usize
    }

    fn overhead_size(&self) -> usize {
        std::mem::size_of::<ShapingCacheKey>()
    }
}

// ShapingCacheKey is no longer needed - goldylox uses String keys directly

/// CacheValue implementation for ShapedText
impl CacheValue for ShapedText {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.runs.len() * std::mem::size_of::<ShapedRun>()
            + self
                .runs
                .iter()
                .map(|run| run.glyphs.len() * std::mem::size_of::<ShapedGlyph>())
                .sum::<usize>()
    }

    fn is_expensive(&self) -> bool {
        // Complex text with many glyphs or runs is expensive to recreate
        self.runs.len() > 1 || self.runs.iter().map(|r| r.glyphs.len()).sum::<usize>() > 50
    }

    fn compression_hint(&self) -> CompressionHint {
        // Large shaped text benefits from compression
        if self.estimated_size() > 4096 {
            CompressionHint::Force
        } else {
            CompressionHint::Disable
        }
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

/// Custom serialization module for unicode_script::Script
mod script_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use unicode_script::Script;

    pub fn serialize<S>(script: &Script, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Script to u16 for serialization
        (*script as u16).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Script, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        // Convert u16 back to Script, defaulting to Latin if invalid
        // Note: Script doesn't have TryFrom<u16>, so we use a match for known values
        let script = match value {
            0 => Script::Latin,
            1 => Script::Greek,
            2 => Script::Cyrillic,
            3 => Script::Arabic,
            4 => Script::Hebrew,
            _ => Script::Latin, // Default fallback
        };
        Ok(script)
    }
}

/// Custom serialization module for unicode_bidi::Level
mod level_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use unicode_bidi::Level;

    pub fn serialize<S>(level: &Level, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Level to u8 for serialization
        level.number().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Level, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        // Convert u8 back to Level, defaulting to LTR if invalid
        Ok(Level::new_explicit(value).unwrap_or(Level::ltr()))
    }
}
