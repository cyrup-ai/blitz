//! Statistics and performance analysis for text rendering
//!
//! This module provides comprehensive statistics collection and analysis capabilities.

use std::sync::atomic::Ordering;
use std::time::Instant;

use super::core::EnhancedTextRenderer;
use super::types::{PerformanceMetrics, RenderMetrics};
use crate::gpu::GpuRenderStats;

impl EnhancedTextRenderer {
    /// Reset all statistics counters
    pub fn reset_stats(&mut self) {
        self.render_passes.store(0, Ordering::Relaxed);
        self.total_glyphs_rendered.store(0, Ordering::Relaxed);
        self.text_areas_processed.store(0, Ordering::Relaxed);
        self.vertex_buffer_reallocations.store(0, Ordering::Relaxed);
        self.preparation_time_ns.store(0, Ordering::Relaxed);
        self.render_time_ns.store(0, Ordering::Relaxed);
        self.current_vertex_buffer_size.store(0, Ordering::Relaxed);
        self.peak_vertex_buffer_size.store(0, Ordering::Relaxed);
        self.last_trim_pass.store(0, Ordering::Relaxed);
        self.stats_reset_time = Instant::now();
    }

    /// Get comprehensive statistics
    pub fn get_stats(&self) -> GpuRenderStats {
        let elapsed_since_reset = self.stats_reset_time.elapsed();
        let render_passes = self.render_passes.load(Ordering::Relaxed);
        let total_glyphs = self.total_glyphs_rendered.load(Ordering::Relaxed);
        let prep_time_ns = self.preparation_time_ns.load(Ordering::Relaxed);
        let render_time_ns = self.render_time_ns.load(Ordering::Relaxed);

        GpuRenderStats {
            render_passes,
            total_glyphs_rendered: total_glyphs,
            text_areas_processed: self.text_areas_processed.load(Ordering::Relaxed),
            atlas_growth_events: 0, // Placeholder
            vertex_buffer_reallocations: self.vertex_buffer_reallocations.load(Ordering::Relaxed),
            avg_glyphs_per_pass: if render_passes > 0 {
                total_glyphs as f64 / render_passes as f64
            } else {
                0.0
            },
            estimated_atlas_memory_bytes: 0, // Placeholder
            average_glyphs_per_pass: if render_passes > 0 {
                total_glyphs as f64 / render_passes as f64
            } else {
                0.0
            },
            average_preparation_time_ms: if render_passes > 0 {
                (prep_time_ns as f64 / render_passes as f64) / 1_000_000.0
            } else {
                0.0
            },
            average_render_time_ms: if render_passes > 0 {
                (render_time_ns as f64 / render_passes as f64) / 1_000_000.0
            } else {
                0.0
            },
            total_preparation_time_ms: prep_time_ns as f64 / 1_000_000.0,
            total_render_time_ms: render_time_ns as f64 / 1_000_000.0,
            current_vertex_buffer_size: self.current_vertex_buffer_size.load(Ordering::Relaxed),
            peak_vertex_buffer_size: self.peak_vertex_buffer_size.load(Ordering::Relaxed),
            uptime_ms: elapsed_since_reset.as_millis() as f64,
            cache_hit_rate: 0.0, // Placeholder for cache statistics
            memory_usage_mb: self.current_vertex_buffer_size.load(Ordering::Relaxed) as f64
                / (1024.0 * 1024.0),
        }
    }

    /// Get detailed performance breakdown
    pub fn get_performance_breakdown(&self) -> (PerformanceMetrics, RenderMetrics) {
        let total_prep = self.preparation_time_ns.load(Ordering::Relaxed);
        let total_render = self.render_time_ns.load(Ordering::Relaxed);
        let elapsed_since_reset = self.stats_reset_time.elapsed();

        let perf_metrics = PerformanceMetrics {
            total_preparation_time_ns: total_prep,
            total_render_time_ns: total_render,
            avg_preparation_time_ns: total_prep / 1.max(1),
            avg_render_time_ns: total_render / 1.max(1),
            preparation_time_ns: total_prep,
            render_time_ns: total_render,
            vertex_buffer_reallocations: self.vertex_buffer_reallocations.load(Ordering::Relaxed),
            current_vertex_buffer_size: self.current_vertex_buffer_size.load(Ordering::Relaxed),
            peak_vertex_buffer_size: self.peak_vertex_buffer_size.load(Ordering::Relaxed),
            stats_duration: elapsed_since_reset,
        };

        let render_metrics = self.render_metrics();

        (perf_metrics, render_metrics)
    }

    /// Check if performance optimization is needed
    pub fn needs_optimization(&self) -> bool {
        let stats = self.get_stats();

        // Check various performance indicators
        stats.average_render_time_ms > self.config.max_render_time_ms
            || stats.vertex_buffer_reallocations > self.config.max_vertex_buffer_reallocations
            || stats.memory_usage_mb > self.config.max_memory_usage_mb
            || (stats.cache_hit_rate < self.config.min_cache_hit_rate && stats.render_passes > 100)
    }

    /// Get optimization recommendations
    pub fn get_optimization_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        let stats = self.get_stats();

        if stats.average_render_time_ms > self.config.max_render_time_ms {
            recommendations.push(format!(
                "Render time ({:.2}ms) exceeds target ({:.2}ms). Consider reducing text complexity or viewport size.",
                stats.average_render_time_ms, self.config.max_render_time_ms
            ));
        }

        if stats.vertex_buffer_reallocations > self.config.max_vertex_buffer_reallocations {
            recommendations.push(format!(
                "High vertex buffer reallocations ({}). Consider pre-allocating larger buffers.",
                stats.vertex_buffer_reallocations
            ));
        }

        if stats.memory_usage_mb > self.config.max_memory_usage_mb {
            recommendations.push(format!(
                "Memory usage ({:.2}MB) exceeds target ({:.2}MB). Consider implementing memory trimming.",
                stats.memory_usage_mb, self.config.max_memory_usage_mb
            ));
        }

        if stats.average_glyphs_per_pass < 10.0 && stats.render_passes > 50 {
            recommendations.push(
                "Low glyph density per render pass. Consider batching text areas for better efficiency.".to_string()
            );
        }

        if recommendations.is_empty() {
            recommendations.push("Performance is within acceptable parameters.".to_string());
        }

        recommendations
    }

    /// Log performance statistics
    pub fn log_stats(&self) {
        let stats = self.get_stats();

        println!(
            "Text Renderer Stats: {} passes, {} glyphs, {:.2}ms avg render, {:.2}MB memory",
            stats.render_passes,
            stats.total_glyphs_rendered,
            stats.average_render_time_ms,
            stats.memory_usage_mb
        );

        if self.needs_optimization() {
            let recommendations = self.get_optimization_recommendations();
            for rec in recommendations {
                println!("Performance recommendation: {}", rec);
            }
        }
    }

    /// Update vertex buffer size tracking
    pub(super) fn update_vertex_buffer_size(&self, new_size: usize) {
        self.current_vertex_buffer_size
            .store(new_size, Ordering::Relaxed);

        // Update peak size if necessary
        let current_peak = self.peak_vertex_buffer_size.load(Ordering::Relaxed);
        if new_size > current_peak {
            self.peak_vertex_buffer_size
                .store(new_size, Ordering::Relaxed);
        }
    }

    /// Record vertex buffer reallocation
    pub(super) fn record_vertex_buffer_reallocation(&self) {
        self.vertex_buffer_reallocations
            .fetch_add(1, Ordering::Relaxed);
    }
}
