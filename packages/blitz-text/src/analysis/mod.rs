//! Lock-free text analysis and script detection for optimal shaping
//!
//! This module provides comprehensive text analysis capabilities with
//! zero-allocation hot paths and thread-local caching for performance.

pub mod analyzer_core;
pub mod bidi_processing;
pub mod caching;
pub mod script_detection;

// Re-export main types for backward compatibility
pub use analyzer_core::TextAnalyzer;
pub use bidi_processing::BidiProcessor;
pub use caching::CacheManager;
pub use script_detection::ScriptDetector;

/// Global analyzer instance for convenience (zero allocation)
static GLOBAL_ANALYZER: once_cell::sync::Lazy<TextAnalyzer> =
    once_cell::sync::Lazy::new(|| TextAnalyzer::new());

/// Convenience function for global analyzer access
#[inline]
pub fn analyze_text_global(
    text: &str,
) -> Result<crate::types::TextAnalysis, crate::error::ShapingError> {
    GLOBAL_ANALYZER.analyze_text(text)
}

/// Convenience function for global cache clearing
pub fn clear_global_caches() {
    GLOBAL_ANALYZER.clear_caches();
}
