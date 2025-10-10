//! Text rendering operations
//!
//! This module handles all text rendering operations, including rendering
//! prepared text and combined measure/prepare/render operations.

use std::time::Instant;

use cosmyc_text::{Attrs, Color};
use glyphon::TextBounds;
use wgpu::{Device, Queue, RenderPass};

use super::system::UnifiedTextSystem;
use crate::text_system::config::{PreparedText, RenderMetrics, TextSystemResult};

impl UnifiedTextSystem {
    /// Render previously prepared text
    pub fn render_prepared(
        &self,
        prepared: &PreparedText,
        render_pass: &mut RenderPass<'_>,
    ) -> TextSystemResult<RenderMetrics> {
        let start_time = Instant::now();

        // Render using GPU components
        let _metrics = self.text_renderer.render_enhanced(
            self.text_atlas.inner(),
            self.viewport.inner(),
            render_pass,
        )?;

        // Track render performance
        self.performance_monitor
            .record_render_time(start_time.elapsed());

        Ok(RenderMetrics {
            total_render_time: start_time.elapsed(),
            text_bounds: prepared.text_area_config.bounds,
            glyph_count: prepared
                .measurement
                .line_measurements
                .iter()
                .map(|line| line.glyph_count)
                .sum(),
        })
    }

    /// Measure, prepare, and render text in one operation
    pub async fn measure_prepare_and_render(
        &mut self,
        device: &Device,
        queue: &Queue,
        render_pass: &mut RenderPass<'_>,
        text: &str,
        attrs: Attrs<'_>,
        position: (f32, f32),
        scale: f32,
        bounds: TextBounds,
        default_color: Color,
        max_width: Option<f32>,
        max_height: Option<f32>,
    ) -> TextSystemResult<RenderMetrics> {
        let prepared = self.measure_and_prepare(
            device,
            queue,
            text,
            attrs,
            position,
            scale,
            bounds,
            default_color,
            max_width,
            max_height,
        ).await?;

        self.render_prepared(&prepared, render_pass)
    }
}
