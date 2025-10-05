//! Bidirectional text processing module
//!
//! This module provides comprehensive BiDi support for mixed LTR/RTL text with:
//! - Unicode Bidirectional Algorithm implementation
//! - Visual ordering and cursor positioning
//! - Text selection across direction boundaries
//! - Lock-free caching for blazing-fast performance
//! - Integration with cosmyc-text shaping system

use cosmyc_text::{FontSystem, Metrics};

pub mod cache;
pub mod cursor;
pub mod multiline;
pub mod processing;
pub mod rendering;
pub mod types;

// Re-export main types and functionality
pub use cache::{CacheManager, CacheMemoryUsage, CacheStatistics};
pub use cursor::{CursorManager, CursorStats};
pub use multiline::{MultiLineBidiProcessor, MultiLineStats};
pub use processing::{BidiProcessor, ProcessingStats};
pub use rendering::{BidiRenderTarget, RenderingStats, TestRenderTarget};
pub use types::{
    BidiError, BidiRenderOptions, BidiSelection, BidiStats, CursorPosition, Direction, LineBidi,
    LineBreakInfo, LineMetrics, MultiLineBidiResult, ParagraphBidi, ProcessedBidi, SelectionRect,
    TextOrientation, UnicodeBidi, VisualRun, WritingMode,
};

/// Lock-free bidirectional text renderer with zero-allocation hot paths
pub struct BidiRenderer {
    processor: BidiProcessor,
    renderer: rendering::BidiRenderer,
    cursor_manager: CursorManager,
    multiline_processor: MultiLineBidiProcessor,
    stats: BidiStats,
}

impl BidiRenderer {
    /// Create new BiDi renderer with optimal defaults
    #[inline]
    pub fn new() -> Self {
        Self {
            processor: BidiProcessor::new(Direction::Auto),
            renderer: rendering::BidiRenderer::new(Direction::Auto),
            cursor_manager: CursorManager::new(Direction::Auto),
            multiline_processor: MultiLineBidiProcessor::new(Direction::Auto, 800.0, 20.0),
            stats: BidiStats {
                total_processed: 0,
                cache_hits: 0,
                cache_misses: 0,
                avg_processing_time_ns: 0,
            },
        }
    }

    /// Set default text direction
    #[inline]
    pub fn set_default_direction(&mut self, direction: Direction) {
        self.processor = BidiProcessor::new(direction);
        self.renderer.set_default_direction(direction);
    }

    /// Enable or disable cache compression
    #[inline]
    pub fn set_cache_compression(&mut self, enable: bool) {
        self.processor.set_cache_compression(enable);
    }

    /// Process bidirectional text with caching
    pub fn process_bidi_text(
        &mut self,
        text: &str,
        options: &BidiRenderOptions,
    ) -> Result<std::sync::Arc<ProcessedBidi>, BidiError> {
        let start_time = std::time::Instant::now();
        let result = self.processor.process_bidi_text(text, options);
        let elapsed = start_time.elapsed().as_nanos() as u64;

        // Update statistics
        self.stats.total_processed += 1;
        self.stats.avg_processing_time_ns = (self.stats.avg_processing_time_ns + elapsed) / 2;

        if result.is_ok() {
            self.stats.cache_hits += 1;
        } else {
            self.stats.cache_misses += 1;
        }

        result
    }

    /// Render bidirectional text to target
    pub fn render_bidi_text(
        &mut self,
        target: &mut dyn BidiRenderTarget,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[crate::types::ShapedRun],
        y_offset: f32,
    ) -> Result<(), BidiError> {
        self.renderer
            .render_bidi_text(target, processed_bidi, shaped_runs, y_offset)
    }

    /// Render multi-line bidirectional text to target
    pub fn render_multiline_bidi_text(
        &mut self,
        target: &mut dyn BidiRenderTarget,
        lines: &[(ProcessedBidi, Vec<crate::types::ShapedRun>)],
        line_height: f32,
    ) -> Result<(), BidiError> {
        self.renderer
            .render_multiline_bidi_text(target, lines, line_height)
    }

    /// Get cursor position from logical text index
    pub fn get_cursor_position(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[crate::types::ShapedRun],
        logical_index: usize,
        line_index: usize,
    ) -> Result<std::sync::Arc<CursorPosition>, BidiError> {
        self.cursor_manager.get_cursor_position(
            processed_bidi,
            shaped_runs,
            logical_index,
            line_index,
        )
    }

    /// Hit test: convert visual coordinates to logical text position
    pub fn hit_test(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[crate::types::ShapedRun],
        x: f32,
        y: f32,
        line_height: f32,
    ) -> Result<CursorPosition, BidiError> {
        self.cursor_manager
            .hit_test(processed_bidi, shaped_runs, x, y, line_height)
    }

    /// Create text selection between two cursor positions
    pub fn create_selection(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[crate::types::ShapedRun],
        start_pos: usize,
        end_pos: usize,
        line_height: f32,
    ) -> Result<BidiSelection, BidiError> {
        self.cursor_manager.create_selection(
            processed_bidi,
            shaped_runs,
            start_pos,
            end_pos,
            line_height as usize,
        )
    }

    /// Process multi-line bidirectional text
    pub fn process_multiline_bidi_text(
        &mut self,
        text: &str,
        options: &BidiRenderOptions,
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<MultiLineBidiResult, BidiError> {
        self.multiline_processor
            .process_multiline_bidi_text(text, options, font_system, metrics)
    }

    /// Wrap text to fit within specified width
    pub fn wrap_text(
        &self,
        text: &str,
        options: &BidiRenderOptions,
        shaped_runs: &[crate::types::ShapedRun],
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<Vec<(ProcessedBidi, Vec<crate::types::ShapedRun>)>, BidiError> {
        self.multiline_processor
            .wrap_text(text, options, shaped_runs, font_system, metrics)
    }

    /// Get BiDi processing statistics
    #[inline]
    pub fn get_stats(&self) -> &BidiStats {
        &self.stats
    }

    /// Clear all BiDi caches
    pub fn clear_caches(&self) -> Result<(), BidiError> {
        CacheManager::clear_all_caches();
        Ok(())
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStatistics {
        CacheManager::get_cache_stats()
    }

    /// Get cache memory usage
    pub fn get_cache_memory_usage(&self) -> CacheMemoryUsage {
        CacheManager::get_cache_memory_usage()
    }
}

impl Default for BidiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if text contains bidirectional content
pub fn has_bidi_content(text: &str) -> bool {
    processing::has_bidi_content(text)
}

/// Get paragraph embedding level for text
pub fn get_paragraph_level(text: &str, base_direction: Direction) -> Result<u8, BidiError> {
    processing::get_paragraph_level(text, base_direction)
}

/// Split text into paragraphs for BiDi processing
pub fn split_paragraphs(text: &str) -> Vec<&str> {
    processing::split_paragraphs(text)
}

/// Validate BiDi processing result
pub fn validate_processed_bidi(processed: &ProcessedBidi) -> Result<(), BidiError> {
    processing::validate_processed_bidi(processed)
}
