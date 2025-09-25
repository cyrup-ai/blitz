//! Text preparation and layout operations
//!
//! This module handles text preparation, layout processing, and glyph preparation.

use std::sync::atomic::Ordering;
use std::time::Instant;

use cosmyc_text::{Buffer, FontSystem, SwashCache};
use glyphon::{PrepareError, TextArea, TextAtlas};
use wgpu::{Device, Queue};

use super::core::EnhancedTextRenderer;
// Performance metrics are handled by goldylox cache system
use crate::gpu::{GpuTextError, GpuTextResult};

impl EnhancedTextRenderer {
    /// Prepare text areas for rendering with comprehensive performance tracking
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        font_system: &mut FontSystem,
        atlas: &mut TextAtlas,
        viewport: &glyphon::Viewport,
        text_areas: &[TextArea],
        swash_cache: &mut SwashCache,
    ) -> Result<(), PrepareError> {
        let start_time = Instant::now();

        // Track text areas processed
        self.text_areas_processed
            .fetch_add(text_areas.len() as u64, Ordering::Relaxed);

        // Count total glyphs to be processed
        let mut total_glyphs = 0;
        for area in text_areas.iter() {
            for run in area.buffer.layout_runs() {
                total_glyphs += run.glyphs.len();
            }
        }

        // Update glyph count
        self.total_glyphs_rendered
            .fetch_add(total_glyphs as u64, Ordering::Relaxed);

        // Prepare using inner renderer - convert slice to iterator (skip in headless mode)
        let result = if let Some(ref mut inner) = self.inner {
            inner.prepare(
                device,
                queue,
                font_system,
                atlas,
                viewport,
                text_areas.iter().cloned(),
                swash_cache,
            )
        } else {
            Ok(()) // Headless mode: no-op
        };

        // Track preparation time
        let elapsed = start_time.elapsed();
        self.preparation_time_ns
            .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);

        result
    }

    /// Prepare text areas with custom glyphs
    pub fn prepare_with_custom_glyphs(
        &mut self,
        device: &Device,
        queue: &Queue,
        font_system: &mut FontSystem,
        atlas: &mut TextAtlas,
        viewport: &glyphon::Viewport,
        text_areas: &[TextArea],
        swash_cache: &mut SwashCache,
    ) -> GpuTextResult<()> {
        let start_time = Instant::now();

        // Process custom glyphs for each text area
        for area in text_areas {
            match self.get_glyphs_for_buffer(&area.buffer) {
                Ok(custom_glyphs) => {
                    if !custom_glyphs.is_empty() {
                        // Process custom glyphs through the renderer
                        for _custom_glyph in custom_glyphs {
                            // Custom glyph processing would go here
                            // This is a placeholder for the actual implementation
                        }
                    }
                }
                Err(e) => {
                    return Err(GpuTextError::CustomGlyph(e.to_string()));
                }
            }
        }

        // Prepare normally
        self.prepare(
            device,
            queue,
            font_system,
            atlas,
            viewport,
            text_areas,
            swash_cache,
        )
        .map_err(|e| GpuTextError::Prepare(e.to_string()))?;

        // Track preparation time
        let elapsed = start_time.elapsed();
        self.preparation_time_ns
            .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);

        Ok(())
    }

    /// Prepare a single buffer for rendering
    pub fn prepare_buffer(
        &mut self,
        device: &Device,
        queue: &Queue,
        font_system: &mut FontSystem,
        atlas: &mut TextAtlas,
        viewport: &glyphon::Viewport,
        buffer: &Buffer,
        bounds: glyphon::TextBounds,
        swash_cache: &mut SwashCache,
    ) -> GpuTextResult<()> {
        let text_area = TextArea {
            buffer,
            left: bounds.left as f32,
            top: bounds.top as f32,
            scale: 1.0,
            bounds,
            default_color: cosmyc_text::Color::rgb(255, 255, 255),
            custom_glyphs: &[],
        };

        self.prepare_with_custom_glyphs(
            device,
            queue,
            font_system,
            atlas,
            viewport,
            &[text_area],
            swash_cache,
        )
    }

    // Performance metrics are provided by goldylox cache system
}
