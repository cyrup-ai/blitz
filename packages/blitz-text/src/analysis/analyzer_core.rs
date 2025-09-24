//! Core text analyzer with comprehensive analysis capabilities
//!
//! This module provides the main TextAnalyzer struct with optimized
//! text analysis, caching, and performance monitoring.

use unicode_bidi::BidiInfo;

use super::bidi_processing::BidiProcessor;
use super::caching::CacheManager;
use super::script_detection::ScriptDetector;
use crate::error::ShapingError;
use crate::types::{BidiRun, TextAnalysis, TextDirection};

/// Lock-free text analyzer with thread-local caching
pub struct TextAnalyzer {
    cache_enabled: bool,
    max_cache_entries: usize,
}

impl Default for TextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextAnalyzer {
    /// Create new text analyzer with optimal defaults
    #[inline]
    pub const fn new() -> Self {
        Self {
            cache_enabled: true,
            max_cache_entries: 1000,
        }
    }

    /// Create analyzer with custom cache settings
    pub const fn with_cache_limit(max_entries: usize) -> Self {
        Self {
            cache_enabled: max_entries > 0,
            max_cache_entries: max_entries,
        }
    }

    /// Comprehensive text analysis with zero-allocation hot paths
    pub fn analyze_text(&self, text: &str) -> Result<TextAnalysis, ShapingError> {
        if text.is_empty() {
            return Ok(TextAnalysis {
                script_runs: Vec::new(),
                base_direction: TextDirection::LeftToRight,
                has_complex_scripts: false,
                requires_bidi: false,
                complexity_score: 0,
            });
        }

        // Check thread-local cache first (zero allocation for cache hits)
        if self.cache_enabled {
            if let Some(cached) = CacheManager::get_cached_analysis(text) {
                return Ok(cached);
            }
        }

        // Perform analysis with optimized allocation patterns
        let script_runs = ScriptDetector::detect_script_runs_optimized(text)?;
        let base_direction = BidiProcessor::determine_base_direction_fast(text);
        let has_complex_scripts = ScriptDetector::has_complex_scripts_fast(&script_runs);
        let requires_bidi = BidiProcessor::requires_bidi_processing_fast(text);
        let complexity_score =
            ScriptDetector::calculate_complexity_score_fast(&script_runs, requires_bidi);

        let analysis = TextAnalysis {
            script_runs,
            base_direction,
            has_complex_scripts,
            requires_bidi,
            complexity_score,
        };

        // Cache result for future use
        if self.cache_enabled {
            CacheManager::cache_analysis(
                text.to_string(),
                analysis.clone(),
                self.max_cache_entries,
            );
        }

        Ok(analysis)
    }

    /// Process bidirectional text with optimized caching
    pub fn process_bidi<'a>(
        &self,
        text: &'a str,
        base_direction: TextDirection,
    ) -> Result<BidiInfo<'a>, ShapingError> {
        BidiProcessor::process_bidi(text, base_direction)
    }

    /// Extract bidi runs from BidiInfo with zero extra allocation
    #[inline]
    pub fn extract_bidi_runs(
        &self,
        bidi_info: &BidiInfo,
        para_range: std::ops::Range<usize>,
    ) -> Vec<BidiRun> {
        BidiProcessor::extract_bidi_runs(bidi_info, para_range)
    }

    /// Language detection based on script with compile-time optimization
    #[inline]
    pub const fn detect_language(
        &self,
        text: &str,
        script: unicode_script::Script,
    ) -> Option<&'static str> {
        ScriptDetector::detect_language(text, script)
    }

    /// Clear all thread-local caches
    pub fn clear_caches(&self) {
        CacheManager::clear_all_caches();
    }

    /// Get cache statistics for monitoring (zero allocation)
    pub fn cache_stats(&self) -> (usize, usize, usize, usize) {
        CacheManager::cache_stats()
    }

    /// Optimize cache sizes based on usage patterns
    pub fn optimize_caches(&mut self) -> Result<(), ShapingError> {
        let (script_size, bidi_size, analysis_size, _) = self.cache_stats();

        // If caches are growing too large, reduce max_cache_entries
        if script_size > 5000 || bidi_size > 5000 || analysis_size > 2000 {
            self.max_cache_entries = self.max_cache_entries.saturating_sub(100).max(100);
        }

        // If caches are small, allow growth
        if script_size < 500 && bidi_size < 500 && analysis_size < 200 {
            self.max_cache_entries = (self.max_cache_entries + 100).min(5000);
        }

        Ok(())
    }

    /// Check if text contains only ASCII characters (ultra-fast path)
    #[inline]
    pub const fn is_ascii_only(text: &str) -> bool {
        ScriptDetector::is_ascii_only(text)
    }

    /// Check if text contains only Latin script (fast path)
    pub fn is_latin_only(&self, text: &str) -> bool {
        ScriptDetector::is_latin_only(text)
    }

    /// Determine if text needs complex processing (optimization hint)
    pub fn needs_complex_processing(&self, text: &str) -> bool {
        // ASCII text never needs complex processing
        if Self::is_ascii_only(text) {
            return false;
        }

        // Check for complex scripts or bidirectional text
        BidiProcessor::requires_bidi_processing_fast(text)
            || ScriptDetector::needs_complex_processing(text)
    }
}
