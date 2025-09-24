//! Utility functions and statistics for text shaping

use std::sync::atomic::Ordering;

use crate::cache::CacheManager;
use crate::analysis::TextAnalyzer;
use crate::error::ShapingError;
use crate::features::FeatureLookup;
use crate::types::{ShapedRun, ShapingContext, TextDirection};

use super::core::{CACHE_HITS, SHAPED_RUNS_BUFFER, GLYPHS_BUFFER, TEXT_RUNS_BUFFER, SHAPING_OPERATIONS, TOTAL_GLYPHS_SHAPED};

/// Fast metrics calculation with compile-time optimization
#[inline]
pub(super) fn calculate_metrics_fast(runs: &[ShapedRun]) -> (f32, f32, f32, usize) {
    if runs.is_empty() {
        return (0.0, 0.0, 0.0, 0);
    }

    let mut total_width = 0.0;
    let mut max_height = 0.0;
    let mut baseline = 0.0;
    let mut i = 0;

    while i < runs.len() {
        total_width += runs[i].width;

        if runs[i].height > max_height {
            max_height = runs[i].height;
        }

        if runs[i].ascent > baseline {
            baseline = runs[i].ascent;
        }

        i += 1;
    }

    let line_count = 1; // Simplified line counting
    (total_width, max_height, baseline, line_count)
}

/// Get shaper statistics (lock-free atomic access)
pub fn get_shaper_stats(cache_manager: &CacheManager, analyzer: &TextAnalyzer) -> ShaperStats {
    ShaperStats {
        shaping_operations: SHAPING_OPERATIONS.load(Ordering::Relaxed),
        cache_hits: CACHE_HITS.load(Ordering::Relaxed),
        total_glyphs_shaped: TOTAL_GLYPHS_SHAPED.load(Ordering::Relaxed),
        cache_stats: cache_manager.stats(),
        analyzer_stats: analyzer.cache_stats(),
    }
}

/// Clear all thread-local caches and buffers
pub fn clear_thread_local_caches() {
    // Clear thread-local buffers
    SHAPED_RUNS_BUFFER.with(|buffer| buffer.borrow_mut().clear());
    GLYPHS_BUFFER.with(|buffer| buffer.borrow_mut().clear());
    TEXT_RUNS_BUFFER.with(|buffer| buffer.borrow_mut().clear());
}

/// Optimize shaper performance based on usage patterns
pub fn optimize_shaper(cache_manager: &mut CacheManager, analyzer: &mut TextAnalyzer) -> Result<(), ShapingError> {
    cache_manager.optimize()?;
    analyzer.optimize_caches()?;
    Ok(())
}

/// Create shaping context for advanced features
#[inline]
pub fn create_shaping_context(
    script: unicode_script::Script,
    language: Option<&'static str>,
    direction: TextDirection,
    font_size: f32,
) -> ShapingContext {
    let features = FeatureLookup::get_features_for_script(script);

    ShapingContext {
        language,
        script,
        direction,
        features,
        font_size,
    }
}

/// Check if shaper needs optimization based on performance metrics
#[inline]
pub fn needs_optimization(stats: &ShaperStats) -> bool {
    let hit_ratio = if stats.shaping_operations > 0 {
        stats.cache_hits as f64 / stats.shaping_operations as f64
    } else {
        1.0
    };

    // Optimize if hit ratio is low or memory usage is high
    hit_ratio < 0.6 || stats.cache_stats.memory_used > 128 * 1024 * 1024
}

/// Comprehensive shaper statistics
#[derive(Debug, Clone)]
pub struct ShaperStats {
    pub shaping_operations: usize,
    pub cache_hits: usize,
    pub total_glyphs_shaped: usize,
    pub cache_stats: crate::cache::CacheStats,
    pub analyzer_stats: (usize, usize, usize, usize),
}

impl ShaperStats {
    /// Calculate cache hit ratio
    #[inline]
    pub fn hit_ratio(&self) -> f64 {
        if self.shaping_operations > 0 {
            self.cache_hits as f64 / self.shaping_operations as f64
        } else {
            0.0
        }
    }

    /// Calculate average glyphs per operation
    #[inline]
    pub fn avg_glyphs_per_operation(&self) -> f64 {
        if self.shaping_operations > 0 {
            self.total_glyphs_shaped as f64 / self.shaping_operations as f64
        } else {
            0.0
        }
    }

    /// Get cache efficiency score (0-100)
    #[inline]
    pub fn efficiency_score(&self) -> f64 {
        let hit_ratio = self.hit_ratio();
        let avg_glyphs = self.avg_glyphs_per_operation();
        
        // Higher hit ratio and reasonable glyph count indicate good efficiency
        let efficiency = (hit_ratio * 0.7) + ((avg_glyphs.min(50.0) / 50.0) * 0.3);
        efficiency * 100.0
    }

    /// Check if performance is good
    #[inline]
    pub fn is_performing_well(&self) -> bool {
        self.hit_ratio() > 0.75 && self.avg_glyphs_per_operation() > 5.0
    }
}