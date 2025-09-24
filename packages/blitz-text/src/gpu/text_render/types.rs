//! Text renderer type definitions
//!
//! This module contains all the type definitions used by the enhanced text renderer system.

/// Metrics for a single render operation
#[derive(Debug, Clone, Copy)]
pub struct RenderMetrics {
    /// Time spent rendering
    pub render_time: std::time::Duration,
    /// Number of glyphs rendered
    pub glyphs_rendered: u32,
    /// Number of draw calls made
    pub draw_calls: u32,
}

/// Detailed performance metrics
#[derive(Debug, Clone, Copy)]
pub struct PerformanceMetrics {
    /// Total time spent in preparation across all calls
    pub total_preparation_time_ns: u64,
    /// Total time spent in rendering across all calls
    pub total_render_time_ns: u64,
    /// Average preparation time per call
    pub avg_preparation_time_ns: u64,
    /// Average render time per call
    pub avg_render_time_ns: u64,
    /// Peak vertex buffer size reached
    pub peak_vertex_buffer_size: usize,
    /// Current vertex buffer size
    pub current_vertex_buffer_size: usize,
    /// Duration since statistics were last reset
    pub stats_duration: std::time::Duration,
    /// Time spent in preparation for current operation
    pub preparation_time_ns: u64,
    /// Time spent in rendering for current operation  
    pub render_time_ns: u64,
    /// Number of vertex buffer reallocations
    pub vertex_buffer_reallocations: u32,
}

impl PerformanceMetrics {
    /// Get preparation throughput (operations per second)
    pub fn preparation_ops_per_second(&self) -> f64 {
        if self.avg_preparation_time_ns > 0 {
            1_000_000_000.0 / self.avg_preparation_time_ns as f64
        } else {
            0.0
        }
    }

    /// Get render throughput (operations per second)
    pub fn render_ops_per_second(&self) -> f64 {
        if self.avg_render_time_ns > 0 {
            1_000_000_000.0 / self.avg_render_time_ns as f64
        } else {
            0.0
        }
    }

    /// Get memory efficiency (current/peak ratio)
    pub fn memory_efficiency(&self) -> f64 {
        if self.peak_vertex_buffer_size > 0 {
            self.current_vertex_buffer_size as f64 / self.peak_vertex_buffer_size as f64
        } else {
            1.0
        }
    }
}
