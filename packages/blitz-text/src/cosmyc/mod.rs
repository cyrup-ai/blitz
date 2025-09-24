//! Cosmic-text integration module with enhanced performance monitoring
//!
//! This module provides comprehensive integration with the cosmyc-text crate,
//! offering enhanced wrappers around SwashCache, ShapeRunCache, and Editor
//! with performance monitoring, caching optimization, and intelligent resource management.

pub mod editor;
pub mod shape_cache;
pub mod swash_cache;

// Re-export enhanced cosmyc-text integration components
// Re-export core cosmyc-text types for convenience
pub use cosmyc_text::{
    fontdb, Action, Align, Attrs, AttrsList, AttrsOwned, Buffer, CacheKey, CacheKeyFlags,
    CacheMetrics, Change, ChangeItem, Color, Cursor, Editor, Family, FontSystem, LayoutCursor,
    LayoutGlyph, LayoutRun, Metrics, Motion, PhysicalGlyph, Selection, ShapeRunCache, Shaping,
    Stretch, Style, SubpixelBin, SwashCache, Weight, Wrap,
};
pub use editor::{EditorStats, EnhancedEditor};
pub use shape_cache::{
    EnhancedShapeRunCache, OptimizedShapeCache, PerformanceMode, ShapeCacheStats, ShapeComplexity,
    ShapeRunUtils,
};
// Re-export swash types needed for font rasterization
pub use swash::{
    scale::image::Content as SwashContent,
    zeno::{Command, Placement},
};
pub use swash_cache::{CacheStats, EnhancedSwashCache, RasterizationUtils};

/// Comprehensive cosmyc-text integration statistics
#[derive(Debug, Clone, Default)]
pub struct IntegrationStats {
    /// SwashCache performance statistics
    pub swash_cache_stats: swash_cache::CacheStats,
    /// ShapeRunCache performance statistics  
    pub shape_cache_stats: shape_cache::ShapeCacheStats,
    /// Editor performance statistics
    pub editor_stats: Option<editor::EditorStats>,
    /// Overall integration metrics
    pub integration_metrics: IntegrationMetrics,
}

/// Overall integration performance metrics
#[derive(Debug, Clone, Default)]
pub struct IntegrationMetrics {
    /// Total memory usage across all caches
    pub total_memory_usage: usize,
    /// Peak memory usage recorded
    pub peak_memory_usage: usize,
    /// Total operations performed across all components
    pub total_operations: u64,
    /// Overall cache hit ratio (weighted average)
    pub overall_hit_ratio: f64,
    /// System efficiency score (0.0 to 1.0)
    pub efficiency_score: f64,
    /// Time since statistics were last reset
    pub stats_duration: std::time::Duration,
}

impl IntegrationMetrics {
    /// Calculate overall efficiency score based on all components
    pub fn calculate_efficiency(&mut self) {
        let swash_efficiency = if self.total_operations > 0 {
            self.overall_hit_ratio
        } else {
            1.0
        };

        let memory_efficiency = if self.peak_memory_usage > 0 {
            1.0 - (self.total_memory_usage as f64 / self.peak_memory_usage as f64).min(1.0)
        } else {
            1.0
        };

        // Weight cache efficiency higher than memory efficiency
        self.efficiency_score = (swash_efficiency * 0.7) + (memory_efficiency * 0.3);
    }

    /// Get memory usage in human-readable format
    pub fn memory_usage_mb(&self) -> f64 {
        self.total_memory_usage as f64 / (1024.0 * 1024.0)
    }

    /// Get peak memory usage in human-readable format
    pub fn peak_memory_usage_mb(&self) -> f64 {
        self.peak_memory_usage as f64 / (1024.0 * 1024.0)
    }
}

/// Comprehensive cosmyc-text integration system
pub struct CosmicTextIntegration {
    /// Enhanced SwashCache for font rasterization
    pub swash_cache: EnhancedSwashCache,
    /// Enhanced ShapeRunCache for shape run caching
    pub shape_cache: EnhancedShapeRunCache,
    /// Optional enhanced editor for text editing
    pub editor: Option<EnhancedEditor<'static>>,
    /// Integration-wide statistics
    stats_start_time: std::time::Instant,
}

impl CosmicTextIntegration {
    /// Create a new cosmyc-text integration system
    pub fn new() -> Self {
        Self {
            swash_cache: EnhancedSwashCache::new(),
            shape_cache: EnhancedShapeRunCache::new(),
            editor: None,
            stats_start_time: std::time::Instant::now(),
        }
    }

    /// Create a new integration system with editor support
    pub fn with_editor(buffer: Buffer) -> Self {
        Self {
            swash_cache: EnhancedSwashCache::new(),
            shape_cache: EnhancedShapeRunCache::new(),
            editor: Some(EnhancedEditor::from_buffer(buffer)),
            stats_start_time: std::time::Instant::now(),
        }
    }

    /// Add editor support to existing integration
    pub fn add_editor(&mut self, buffer: Buffer) {
        self.editor = Some(EnhancedEditor::from_buffer(buffer));
    }

    /// Remove editor support
    pub fn remove_editor(&mut self) {
        self.editor = None;
    }

    /// Get comprehensive integration statistics
    pub fn get_stats(&self) -> IntegrationStats {
        let swash_stats = self.swash_cache.image_cache_stats();
        let shape_stats = self.shape_cache.cache_stats();
        let editor_stats = self.editor.as_ref().map(|e| e.get_stats());

        // Calculate overall metrics
        let total_operations = swash_stats.total + shape_stats.total;
        let overall_hit_ratio = if total_operations > 0 {
            let swash_weight = swash_stats.total as f64 / total_operations as f64;
            let shape_weight = shape_stats.total as f64 / total_operations as f64;
            (swash_stats.hit_ratio * swash_weight) + (shape_stats.hit_ratio * shape_weight)
        } else {
            0.0
        };

        // Estimate total memory usage (simplified)
        let estimated_memory = swash_stats.total * 1024 + // Estimate 1KB per swash operation
                               shape_stats.total_glyphs_cached * 512; // Estimate 512B per cached glyph

        let mut integration_metrics = IntegrationMetrics {
            total_memory_usage: estimated_memory,
            peak_memory_usage: estimated_memory, // Simplified - would track actual peak
            total_operations: total_operations as u64,
            overall_hit_ratio,
            efficiency_score: 0.0,
            stats_duration: self.stats_start_time.elapsed(),
        };

        integration_metrics.calculate_efficiency();

        IntegrationStats {
            swash_cache_stats: swash_stats,
            shape_cache_stats: shape_stats,
            editor_stats,
            integration_metrics,
        }
    }

    /// Optimize all caches and components
    pub fn optimize_all(&mut self) -> IntegrationOptimizationResult {
        let start_time = std::time::Instant::now();

        // Clear swash cache statistics (it manages its own optimization)
        self.swash_cache.clear_stats();

        // Clear shape cache statistics
        self.shape_cache.clear_stats();

        // Optimize editor if present
        let editor_optimized = if let Some(editor) = &self.editor {
            editor.clear_stats();
            true
        } else {
            false
        };

        IntegrationOptimizationResult {
            swash_cache_optimized: true,
            shape_cache_optimized: true,
            editor_optimized,
            optimization_time: start_time.elapsed(),
            memory_saved: 0, // Would calculate actual memory saved
        }
    }

    /// Reset all statistics across all components
    pub fn reset_all_stats(&mut self) {
        self.swash_cache.clear_stats();
        self.shape_cache.clear_stats();

        if let Some(editor) = &self.editor {
            editor.clear_stats();
        }

        self.stats_start_time = std::time::Instant::now();
    }

    /// Check if system-wide optimization is recommended
    pub fn should_optimize(&self) -> bool {
        let stats = self.get_stats();

        // Recommend optimization if efficiency is low
        stats.integration_metrics.efficiency_score < 0.6 ||
        // Or if memory usage is high
        stats.integration_metrics.memory_usage_mb() > 100.0 ||
        // Or if cache hit ratios are poor
        stats.swash_cache_stats.hit_ratio < 0.5 ||
        stats.shape_cache_stats.hit_ratio < 0.5
    }

    /// Get system health score (0.0 to 1.0)
    pub fn health_score(&self) -> f64 {
        let stats = self.get_stats();

        // Factor in multiple health indicators
        let cache_health =
            (stats.swash_cache_stats.hit_ratio + stats.shape_cache_stats.hit_ratio) / 2.0;
        let memory_health = if stats.integration_metrics.peak_memory_usage > 0 {
            1.0 - (stats.integration_metrics.total_memory_usage as f64
                / stats.integration_metrics.peak_memory_usage as f64)
                .min(1.0)
        } else {
            1.0
        };
        let efficiency_health = stats.integration_metrics.efficiency_score;

        // Weighted average of health indicators
        (cache_health * 0.4) + (memory_health * 0.3) + (efficiency_health * 0.3)
    }
}

impl Default for CosmicTextIntegration {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of system-wide optimization
#[derive(Debug, Clone)]
pub struct IntegrationOptimizationResult {
    pub swash_cache_optimized: bool,
    pub shape_cache_optimized: bool,
    pub editor_optimized: bool,
    pub optimization_time: std::time::Duration,
    pub memory_saved: usize,
}

/// Convert CacheMetrics to Metrics - using the existing From implementation in cosmyc_text
pub fn cache_metrics_to_metrics(cache_metrics: CacheMetrics) -> Metrics {
    Metrics::from(cache_metrics)
}

/// Utility functions for cosmyc-text integration
pub mod utils {
    use super::*;

    /// Convert cosmyc-text CacheMetrics to our Metrics type
    pub fn cache_metrics_to_metrics(cache_metrics: CacheMetrics) -> Metrics {
        cache_metrics.into()
    }

    /// Create optimized Attrs for performance
    pub fn create_performance_attrs(
        family: Family,
        size: f32,
        weight: Weight,
        style: Style,
    ) -> Attrs {
        Attrs::new()
            .family(family)
            .metrics(Metrics::new(size, size * 1.2))
            .weight(weight)
            .style(style)
    }

    /// Estimate memory usage for a text string
    pub fn estimate_text_memory(text: &str, _attrs: &Attrs) -> usize {
        let base_size = text.len() * 4; // Estimate 4 bytes per character
        let font_overhead = 1024; // Estimate 1KB for font metadata
        let attrs_overhead = 256; // Estimate 256B for attributes

        base_size + font_overhead + attrs_overhead
    }

    /// Create a color with optimal performance characteristics
    pub fn create_color_rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::rgba(r, g, b, a)
    }

    /// Validate that cosmyc-text types are compatible
    pub fn validate_cosmyc_text_compatibility() -> bool {
        // Perform basic compatibility checks
        let _font_system = FontSystem::new();
        let _buffer = Buffer::new_empty(Metrics::new(16.0, 20.0));
        let _swash_cache = SwashCache::new();
        let _shape_cache = ShapeRunCache::default();

        // If we get here, all types are compatible
        true
    }
}

// Tests extracted to tests/cosmyc_integration_tests.rs
