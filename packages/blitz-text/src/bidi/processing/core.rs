//! BiDi text processing core
//!
//! This module contains the main BidiProcessor struct and core processing methods.

use std::sync::Arc;

use unicode_bidi::BidiInfo;

use super::super::cache::{CacheManager, BIDI_CACHE_HITS, BIDI_CACHE_MISSES};
use super::super::types::{BidiCacheKey, BidiError, BidiRenderOptions, Direction, ProcessedBidi};
use super::analysis::BidiAnalyzer;
use super::direction::DirectionDetector;
use super::validation::ProcessingStats;

/// Core BiDi text processor
pub struct BidiProcessor {
    default_direction: Direction,
    enable_cache_compression: bool,
    direction_detector: DirectionDetector,
    analyzer: BidiAnalyzer,
}

impl BidiProcessor {
    /// Create new BiDi processor
    pub fn new(default_direction: Direction) -> Self {
        Self {
            default_direction,
            enable_cache_compression: false,
            direction_detector: DirectionDetector::new(default_direction),
            analyzer: BidiAnalyzer::new(),
        }
    }

    /// Set cache compression option
    pub fn set_cache_compression(&mut self, enable: bool) {
        self.enable_cache_compression = enable;
    }

    /// Process bidirectional text with caching
    pub fn process_bidi_text(
        &self,
        text: &str,
        options: &BidiRenderOptions,
    ) -> Result<Arc<ProcessedBidi>, BidiError> {
        let cache_key = BidiCacheKey::new(text, options.base_direction);

        // Try cache first
        if let Some(cached) = CacheManager::get_bidi_cached(&cache_key) {
            return Ok(cached);
        }

        // Process uncached
        let result = self.process_bidi_text_uncached(text, options)?;
        let arc_result = Arc::new(result);

        // Store in cache
        CacheManager::store_bidi_cached(cache_key, arc_result.clone());

        Ok(arc_result)
    }

    /// Process BiDi text without caching (internal)
    fn process_bidi_text_uncached(
        &self,
        text: &str,
        options: &BidiRenderOptions,
    ) -> Result<ProcessedBidi, BidiError> {
        if text.is_empty() {
            return Ok(ProcessedBidi {
                text: String::new(),
                visual_runs: Vec::new(),
                logical_to_visual: Vec::new(),
                visual_to_logical: Vec::new(),
                base_direction: options.base_direction,
                paragraph_level: 0,
            });
        }

        // Determine base direction
        let base_direction = self
            .direction_detector
            .determine_base_direction(text, options)?;
        let base_level = self.direction_detector.direction_to_level(base_direction)?;

        // Create BiDi info using unicode-bidi crate
        let bidi_info = BidiInfo::new(text, Some(base_level));

        // Process all paragraphs for production-quality multi-paragraph BiDi support
        let (visual_runs, logical_to_visual, visual_to_logical) =
            self.analyzer.process_all_paragraphs(text, &bidi_info)?;

        // Use the first paragraph's level as the overall paragraph level
        let paragraph_level = bidi_info
            .paragraphs
            .first()
            .map(|p| p.level.number())
            .unwrap_or(base_level.number());

        Ok(ProcessedBidi {
            text: text.to_string(),
            visual_runs,
            logical_to_visual,
            visual_to_logical,
            base_direction,
            paragraph_level,
        })
    }

    /// Get processing statistics
    pub fn get_processing_stats() -> ProcessingStats {
        ProcessingStats {
            cache_hits: BIDI_CACHE_HITS.load(std::sync::atomic::Ordering::Relaxed) as u64,
            cache_misses: BIDI_CACHE_MISSES.load(std::sync::atomic::Ordering::Relaxed) as u64,
        }
    }
}
