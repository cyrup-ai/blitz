//! Text rendering and metrics operations
//!
//! This module handles the actual rendering of text and performance metrics collection.

use std::sync::atomic::Ordering;
use std::time::Instant;

use glyphon::{RenderError, TextBounds, Viewport};
use wgpu::RenderPass;

use super::core::EnhancedTextRenderer;
use super::types::{PerformanceMetrics, RenderMetrics};
use crate::gpu::GpuTextResult;

impl EnhancedTextRenderer {
    /// Render text with comprehensive performance tracking
    pub fn render<'pass>(
        &'pass self,
        atlas: &'pass glyphon::TextAtlas,
        viewport: Viewport,
        pass: &mut RenderPass<'pass>,
    ) -> Result<(), RenderError> {
        let start_time = Instant::now();

        // Increment render pass counter
        self.render_passes.fetch_add(1, Ordering::Relaxed);

        // Perform the actual rendering
        let result = self.inner.render(atlas, &viewport, pass);

        // Track render time
        let elapsed = start_time.elapsed();
        self.render_time_ns
            .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);

        result
    }

    /// Render text with bounds checking
    pub fn render_with_bounds<'pass>(
        &'pass self,
        atlas: &'pass glyphon::TextAtlas,
        viewport: Viewport,
        bounds: TextBounds,
        pass: &mut RenderPass<'pass>,
    ) -> Result<(), RenderError> {
        // Validate bounds are within viewport
        if bounds.left < 0
            || bounds.top < 0
            || bounds.right > viewport.resolution().width as i32
            || bounds.bottom > viewport.resolution().height as i32
        {
            // Clip bounds to viewport
            let clipped_bounds = TextBounds {
                left: bounds.left.max(0),
                top: bounds.top.max(0),
                right: bounds.right.min(viewport.resolution().width as i32),
                bottom: bounds.bottom.min(viewport.resolution().height as i32),
            };

            // Only render if there's still visible area
            if clipped_bounds.right > clipped_bounds.left
                && clipped_bounds.bottom > clipped_bounds.top
            {
                self.render(atlas, viewport, pass)
            } else {
                Ok(())
            }
        } else {
            self.render(atlas, viewport, pass)
        }
    }

    // GPU render metrics are tracked by goldylox telemetry system

    /// Get rendering metrics
    pub fn render_metrics(&self) -> RenderMetrics {
        let elapsed_since_reset = self.stats_reset_time.elapsed();

        RenderMetrics {
            render_time: elapsed_since_reset,
            glyphs_rendered: self.total_glyphs_rendered.load(Ordering::Relaxed) as u32,
            draw_calls: self.render_passes.load(Ordering::Relaxed) as u32,
        }
    }

    /// Get performance metrics
    pub fn performance_metrics(&self) -> PerformanceMetrics {
        let elapsed_since_reset = self.stats_reset_time.elapsed();
        let total_prep_time = self.preparation_time_ns.load(Ordering::Relaxed);
        let total_render_time = self.render_time_ns.load(Ordering::Relaxed);

        PerformanceMetrics {
            total_preparation_time_ns: total_prep_time,
            total_render_time_ns: total_render_time,
            avg_preparation_time_ns: total_prep_time / 1.max(1),
            avg_render_time_ns: total_render_time / 1.max(1),
            preparation_time_ns: total_prep_time,
            render_time_ns: total_render_time,
            vertex_buffer_reallocations: self.vertex_buffer_reallocations.load(Ordering::Relaxed),
            current_vertex_buffer_size: self.current_vertex_buffer_size.load(Ordering::Relaxed),
            peak_vertex_buffer_size: self.peak_vertex_buffer_size.load(Ordering::Relaxed),
            stats_duration: elapsed_since_reset,
        }
    }

    /// Calculate rendering efficiency metrics
    pub fn efficiency_metrics(&self) -> GpuTextResult<(f64, f64, f64)> {
        let render_metrics = self.render_metrics();
        let perf_metrics = self.performance_metrics();

        // Glyphs per millisecond
        let glyph_throughput = if render_metrics.render_time.as_millis() > 0 {
            render_metrics.glyphs_rendered as f64 / render_metrics.render_time.as_millis() as f64
        } else {
            0.0
        };

        // Memory efficiency (glyphs per KB of vertex buffer)
        let memory_efficiency = if perf_metrics.current_vertex_buffer_size > 0 {
            render_metrics.glyphs_rendered as f64
                / (perf_metrics.current_vertex_buffer_size as f64 / 1024.0)
        } else {
            0.0
        };

        // Overall efficiency score (composite metric)
        let efficiency_score = if perf_metrics.stats_duration.as_millis() > 0 {
            render_metrics.glyphs_rendered as f64 / perf_metrics.stats_duration.as_millis() as f64
        } else {
            0.0
        };

        Ok((glyph_throughput, memory_efficiency, efficiency_score))
    }
}
