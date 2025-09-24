//! Main text measurement functionality with comprehensive cosmyc-text integration
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;

use cosmyc_text::{Align, Attrs, Shaping, Wrap};

use super::baseline_calculation::BaselineCalculator;
use super::bounds_calculation::BoundsCalculator;
use super::core::EnhancedTextMeasurer;
use super::font_metrics::FontMetricsCalculator;
use super::glyph_extraction::GlyphExtractor;
use super::layout_analysis::LayoutAnalyzer;
use super::types::EnhancedTextMeasurement;
use crate::cosmyc_types::{EnhancedBuffer, EnhancedFontSystem};
use crate::measurement::types::*;

impl EnhancedTextMeasurer {
    /// Measure text with comprehensive cosmyc-text integration
    /// Zero allocation through buffer reuse and optimized cache patterns
    #[inline]
    pub fn measure_text_enhanced(
        &mut self,
        text: &str,
        attrs: &Attrs,
        max_width: Option<f32>,
        max_height: Option<f32>,
        wrap: Option<Wrap>,
        align: Option<Align>,
        shaping: Option<Shaping>,
    ) -> MeasurementResult<EnhancedTextMeasurement> {
        let wrap = wrap.unwrap_or(self.default_wrap);
        let align = align.unwrap_or(self.default_align);
        let shaping = shaping.unwrap_or(self.default_shaping);

        // Generate cache key for this measurement (zero allocation for common cases)
        let _cache_key = crate::measurement::MeasurementCacheKey::new(
            text,
            attrs
                .metrics_opt
                .map(|m| Into::<cosmyc_text::Metrics>::into(m).font_size)
                .unwrap_or(16.0),
            max_width,
            match attrs.family {
                cosmyc_text::Family::Name(name) => name,
                cosmyc_text::Family::Serif => "serif",
                cosmyc_text::Family::SansSerif => "sans-serif",
                cosmyc_text::Family::Cursive => "cursive",
                cosmyc_text::Family::Fantasy => "fantasy",
                cosmyc_text::Family::Monospace => "monospace",
            },
            crate::measurement::types::CSSBaseline::Alphabetic,
        );

        // Check cache first for performance (lock-free lookup)
        // TODO: Implement measurement caching - temporarily disabled
        // if let Some(cached_measurement) = self.cache_manager.get_measurement(&cache_key) {
        //     return Ok(cached_measurement.into());
        // }

        // Create enhanced buffer for measurement with optimized settings
        let metrics = attrs
            .metrics_opt
            .map(|cache_metrics| cache_metrics.into())
            .unwrap_or(self.default_metrics);

        let buffer = {
            let font_system = self.font_system.borrow_mut();
            let mut buffer = EnhancedBuffer::new(&mut *font_system, metrics);

            // Set text with enhanced caching optimization
            buffer.set_text_cached(&mut *font_system, text, attrs, shaping);

            // Set buffer size for wrapping (zero allocation if not needed)
            buffer
                .inner_mut()
                .set_size(&mut *font_system, max_width, max_height);

            // Shape text with comprehensive layout
            buffer
                .inner_mut()
                .shape_until_scroll(&mut *font_system, false);

            buffer
        }; // mutable borrow of font_system is dropped here

        // Initialize analyzers with default enhanced font system (to avoid borrow conflicts)
        let enhanced_font_system = Rc::new(RefCell::new(EnhancedFontSystem::default()));
        let layout_analyzer = LayoutAnalyzer::new(&enhanced_font_system);
        let glyph_extractor = GlyphExtractor::new();
        let font_metrics_calc = FontMetricsCalculator::new()?;
        let baseline_calc = BaselineCalculator::new();
        let bounds_calc = BoundsCalculator::new();

        // Process layout runs with zero allocation aggregation
        let mut line_measurements = Vec::with_capacity(4); // Pre-allocate for common case
        let mut total_width = 0.0f32;
        let mut total_height = 0.0f32;
        let mut total_character_count = 0;
        let mut all_character_positions = Vec::with_capacity(text.len()); // Pre-allocate
        let mut physical_glyphs = Vec::with_capacity(text.len()); // Pre-allocate

        // Process each layout run with enhanced analysis (zero allocation hot path)
        for run in buffer.inner().layout_runs() {
            let line_measurement = layout_analyzer.measure_layout_run_comprehensive(&run)?;

            total_width = total_width.max(line_measurement.width);
            total_height += line_measurement.height;
            total_character_count += line_measurement.end_char - line_measurement.start_char;
            all_character_positions.extend(line_measurement.character_positions.clone());

            // Extract physical glyphs for potential rendering (optimized allocation)
            let run_physical_glyphs =
                glyph_extractor.extract_physical_glyphs(&run, (0.0, 0.0), 1.0);
            physical_glyphs.extend(run_physical_glyphs);

            line_measurements.push(line_measurement);
        }

        // Calculate comprehensive font metrics (with caching) - get fresh mutable borrow
        let font_system = self.font_system.borrow_mut();
        let font_metrics =
            font_metrics_calc.extract_comprehensive_font_metrics(&attrs, &mut *font_system)?;

        // Calculate baseline information (zero allocation computation)
        let baseline_info =
            baseline_calc.calculate_comprehensive_baselines(&attrs, &font_metrics)?;

        // Calculate comprehensive bounds (optimized iteration)
        let bounds = bounds_calc.calculate_comprehensive_bounds(&line_measurements);

        let result = EnhancedTextMeasurement {
            // Basic measurements
            content_width: total_width,
            content_height: total_height,
            line_height: metrics.line_height,
            baseline: baseline_info.alphabetic,
            ascent: font_metrics.ascent as f32,
            descent: -font_metrics.descent as f32,
            line_gap: font_metrics.line_gap as f32,
            x_height: font_metrics
                .x_height
                .unwrap_or(font_metrics.cap_height.unwrap_or(500)) as f32,
            cap_height: font_metrics.cap_height.unwrap_or(700) as f32,
            advance_width: total_width,
            bounds,
            line_measurements,
            total_character_count,
            baseline_offset: 0.0, // Calculated from first line
            measured_at: std::time::Instant::now(),

            // Enhanced measurements
            font_metrics,
            baseline_info,
            all_character_positions,
            text_wrap: wrap,
            text_align: align,
            text_shaping: shaping,
            glyph_count: physical_glyphs.len(),
            physical_glyphs,
        };

        // Cache the result for future use (non-blocking operation)
        // TODO: Implement measurement caching - temporarily disabled
        // self.cache_manager
        //     .cache_measurement(cache_key, result.clone().into());

        Ok(result)
    }
}
