//! Advanced text shaping pipeline with complex script support
//!
//! This module provides comprehensive text shaping capabilities for international
//! typography, including bidirectional text, complex scripts, and advanced
//! OpenType features.

// Public modules
pub mod analysis;
pub mod features;
pub mod implementation;
pub mod types;

// Re-export all public types and functions to maintain API compatibility
pub use analysis::{analyze_text_comprehensive, process_bidi_optimized};
pub use features::{advanced_features, get_script_features, script_utils, DEFAULT_FEATURES};
pub use implementation::TextShaper;
pub use types::{
    FeatureSettings, GlyphFlags, ScriptComplexity, ScriptRun, ShapedGlyph, ShapedRun, ShapedText,
    ShapingCacheKey, TextAnalysis, TextDirection, TextRun,
};

// Re-export the error type for convenience
pub use crate::error::ShapingError;
