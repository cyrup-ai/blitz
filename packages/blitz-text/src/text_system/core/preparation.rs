//! Text preparation and buffer management
//!
//! This module handles the complex process of preparing text for GPU rendering,
//! including buffer creation, text shaping, and GPU preparation.

use std::time::Instant;

use cosmyc_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping};
use glyphon::{Resolution, TextArea, TextBounds};
use wgpu::{Device, Queue};

use super::UnifiedTextSystem;
use crate::text_system::config::{PreparedText, TextAreaConfig, TextSystemResult};

impl UnifiedTextSystem {
    /// Measure and prepare text for GPU rendering
    pub fn measure_and_prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        text: &str,
        attrs: Attrs,
        position: (f32, f32),
        scale: f32,
        bounds: TextBounds,
        default_color: Color,
        max_width: Option<f32>,
        max_height: Option<f32>,
    ) -> TextSystemResult<PreparedText> {
        let start_time = Instant::now();

        // Step 1: Measure text layout
        let measurement = self.measure_text(text, attrs.clone(), max_width, max_height)?;

        // Step 2: Get custom glyphs first (before borrowing font_system)
        let custom_glyphs = self
            .get_custom_glyphs_for_text_range(text, 0..text.len())
            .unwrap_or_else(|_| Vec::new());

        // Step 3: Create buffer for GPU rendering - lock-free per-thread access
        let font_system_cell = self
            .font_system
            .get_or(|| std::cell::RefCell::new(FontSystem::new()));
        let mut font_system = font_system_cell.borrow_mut();

        let metrics = attrs
            .metrics_opt
            .map(|cache_metrics| cache_metrics.into())
            .unwrap_or_else(|| Metrics::new(16.0, 20.0));

        let mut buffer = Buffer::new(&mut *font_system, metrics);
        let spans = std::iter::once((text, attrs.clone()));
        buffer.set_rich_text(&mut *font_system, spans, &attrs, Shaping::Advanced, None);

        if let Some(width) = max_width {
            buffer.set_size(&mut *font_system, Some(width), max_height);
        }

        buffer.shape_until_scroll(&mut *font_system, true);

        let text_area = TextArea {
            buffer: &buffer,
            left: position.0,
            top: position.1,
            scale,
            bounds,
            default_color,
            custom_glyphs: &custom_glyphs,
        };

        // Update viewport if needed
        let viewport_resolution = Resolution {
            width: (bounds.right - bounds.left).max(0) as u32,
            height: (bounds.bottom - bounds.top).max(0) as u32,
        };
        self.viewport.update_enhanced(queue, viewport_resolution)?;

        // Prepare for rendering (skip if in headless mode)
        if let (Some(atlas), Some(viewport)) = (self.text_atlas.inner_mut(), self.viewport.inner()) {
            self.text_renderer.prepare_enhanced(
                device,
                queue,
                &mut *font_system,
                atlas,
                viewport,
                std::iter::once(text_area),
                self.cosmyc_integration.swash_cache.inner_mut(),
            )?;
        }

        // Track preparation performance
        self.performance_monitor
            .record_preparation_time(start_time.elapsed());

        Ok(PreparedText {
            measurement,
            buffer,
            text_area_config: TextAreaConfig {
                position,
                scale,
                bounds,
                default_color,
            },
            preparation_time: start_time.elapsed(),
        })
    }
}
