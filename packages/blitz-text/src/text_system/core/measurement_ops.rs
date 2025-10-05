//! Measurement operations for the unified text system
//!
//! This module contains measurement-specific operations that were extracted
//! from the original core implementation for better modularity.

use std::cell::RefCell;
use std::time::Instant;

use cosmyc_text::{Attrs, Buffer, FontSystem, Metrics};

use super::UnifiedTextSystem;
use crate::measurement::TextMeasurement;
use crate::text_system::config::TextSystemResult;

impl UnifiedTextSystem {
    /// Measure text with advanced typography features
    pub fn measure_text_advanced(
        &mut self,
        text: &str,
        attrs: Attrs,
        _max_width: Option<f32>,
        _max_height: Option<f32>,
    ) -> TextSystemResult<TextMeasurement> {
        let start_time = Instant::now();

        // Get thread-local font system
        let font_system = self.font_system.get_or(|| RefCell::new(FontSystem::new()));
        let mut font_system = font_system.borrow_mut();

        // Create temporary buffer for measurement
        let mut buffer = Buffer::new(&mut *font_system, Metrics::new(14.0, 20.0));
        buffer.set_text(
            &mut *font_system,
            text,
            &attrs,
            cosmyc_text::Shaping::Advanced,
        );

        // Run layout
        buffer.shape_until_scroll(&mut *font_system, false);

        let measurement_time = start_time.elapsed();

        // Extract measurements from buffer
        let lines: Vec<_> = buffer.layout_runs().collect();
        let total_height = lines.len() as f32 * buffer.metrics().line_height;
        let max_width_actual = lines.iter().map(|line| line.line_w).fold(0.0f32, f32::max);

        // Update performance stats
        self.performance_monitor
            .record_measurement_time(measurement_time);

        Ok(TextMeasurement {
            content_width: max_width_actual,
            content_height: total_height,
            line_height: buffer.metrics().line_height,
            baseline: buffer.metrics().line_height * 0.8,
            ascent: buffer.metrics().line_height * 0.8,
            descent: buffer.metrics().line_height * 0.2,
            line_gap: 0.0,
            x_height: buffer.metrics().line_height * 0.5,
            cap_height: buffer.metrics().line_height * 0.7,
            advance_width: max_width_actual,
            bounds: crate::measurement::types::bounds_types::TextBounds::default(),
            line_measurements: Vec::new(),
            total_character_count: text.len(),
            baseline_offset: 0.0,
            measured_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }

    /// Quick measurement for simple text
    pub fn measure_text_simple(
        &mut self,
        text: &str,
        font_size: f32,
        max_width: Option<f32>,
    ) -> TextSystemResult<TextMeasurement> {
        let attrs = Attrs::new().metadata(font_size as usize);

        self.measure_text_advanced(text, attrs, max_width, None)
    }

    /// Measure single line of text
    pub fn measure_line(&mut self, text: &str, attrs: Attrs) -> TextSystemResult<f32> {
        let request = crate::measurement::types::measurement_request::MeasurementRequest {
            text: text.to_string(),
            font_id: 0, // Default font ID
            font_size: attrs
                .metrics_opt
                .map(|m| cosmyc_text::Metrics::from(m).font_size)
                .unwrap_or(14.0),
            max_width: None,
            enable_shaping: true,
            language: None,
            direction: None,
        };
        let measurement = self
            .text_measurer
            .measure_text(&request)
            .map_err(|e| crate::text_system::config::TextSystemError::Measurement(e))?;
        Ok(measurement.content_width)
    }

    /// Get metrics for current font configuration
    pub fn get_font_metrics(&self, font_size: f32) -> Metrics {
        Metrics::new(font_size, font_size * 1.4)
    }
}
