//! Measurement operations for the unified text system
//!
//! This module contains measurement-specific operations that were extracted
//! from the original core implementation for better modularity.

use cosmyc_text::{Attrs, Metrics};

use super::UnifiedTextSystem;
use crate::measurement::TextMeasurement;
use crate::text_system::config::TextSystemResult;

impl UnifiedTextSystem {
    /// Measure text with advanced typography features
    pub async fn measure_text_advanced(
        &self,
        text: &str,
        attrs: Attrs<'_>,
        max_width: Option<f32>,
        _max_height: Option<f32>,
    ) -> TextSystemResult<TextMeasurement> {
        use crate::measurement::types::measurement_request::TextDirection;
        
        // Extract font family from attrs
        let font_family = Some(match attrs.family {
            cosmyc_text::Family::Name(name) => name.to_string(),
            cosmyc_text::Family::Serif => "serif".to_string(),
            cosmyc_text::Family::SansSerif => "sans-serif".to_string(),
            cosmyc_text::Family::Monospace => "monospace".to_string(),
            cosmyc_text::Family::Cursive => "cursive".to_string(),
            cosmyc_text::Family::Fantasy => "fantasy".to_string(),
        });

        // Extract font size from attrs
        let font_size = attrs
            .metrics_opt
            .map(|m| cosmyc_text::Metrics::from(m).font_size)
            .unwrap_or(14.0);

        // Build measurement request
        let request = crate::measurement::types::measurement_request::MeasurementRequest {
            text: text.to_string(),
            font_id: 0,
            font_size,
            max_width,
            enable_shaping: true,
            language: None,
            direction: Some(TextDirection::Auto),
            font_family,
        };

        // Delegate to proper measurement system (uses perform_measurement internally)
        self.text_measurer
            .measure_text(&request).await
            .map_err(|e| crate::text_system::config::TextSystemError::Measurement(e))
    }

    /// Quick measurement for simple text
    pub async fn measure_text_simple(
        &self,
        text: &str,
        font_size: f32,
        max_width: Option<f32>,
    ) -> TextSystemResult<TextMeasurement> {
        // Use proper Metrics construction
        let attrs = Attrs::new().metrics(cosmyc_text::Metrics::new(font_size, font_size * 1.4));

        // Delegate to measure_text (not measure_text_advanced)
        self.measure_text(text, attrs, max_width, None).await
    }

    /// Measure single line of text
    pub async fn measure_line(&self, text: &str, attrs: Attrs<'_>) -> TextSystemResult<f32> {
        use crate::measurement::types::measurement_request::TextDirection;
        
        let font_family = Some(match attrs.family {
            cosmyc_text::Family::Name(name) => name.to_string(),
            cosmyc_text::Family::Serif => "serif".to_string(),
            cosmyc_text::Family::SansSerif => "sans-serif".to_string(),
            cosmyc_text::Family::Monospace => "monospace".to_string(),
            cosmyc_text::Family::Cursive => "cursive".to_string(),
            cosmyc_text::Family::Fantasy => "fantasy".to_string(),
        });
        
        let request = crate::measurement::types::measurement_request::MeasurementRequest {
            text: text.to_string(),
            font_id: 0,
            font_size: attrs
                .metrics_opt
                .map(|m| cosmyc_text::Metrics::from(m).font_size)
                .unwrap_or(14.0),
            max_width: None,
            enable_shaping: true,
            language: None,
            direction: Some(TextDirection::Auto),
            font_family,
        };
        let measurement = self
            .text_measurer
            .measure_text(&request).await
            .map_err(|e| crate::text_system::config::TextSystemError::Measurement(e))?;
        Ok(measurement.content_width)
    }

    /// Get metrics for current font configuration (uses standard 1.4 line height multiplier)
    pub fn get_font_metrics(&self, font_size: f32) -> Metrics {
        Metrics::new(font_size, font_size * 1.4)  // Standard CSS default
    }
}
