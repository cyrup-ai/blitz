//! Comprehensive baseline calculations for all CSS baseline types with zero allocation

use cosmyc_text::Attrs;

use super::types::BaselineInfo;
use crate::measurement::types::{FontMetrics, MeasurementResult};

/// Baseline calculator for comprehensive CSS baseline support with optimized calculations
pub struct BaselineCalculator;

impl BaselineCalculator {
    /// Create new baseline calculator (zero allocation)
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Calculate comprehensive baseline information for all CSS baseline types
    /// Optimized for zero allocation and maximum performance
    #[inline]
    pub fn calculate_comprehensive_baselines(
        &self,
        attrs: &Attrs,
        font_metrics: &FontMetrics,
    ) -> MeasurementResult<BaselineInfo> {
        // Extract font size once for efficiency
        let font_size = attrs
            .metrics_opt
            .map(|m| Into::<cosmyc_text::Metrics>::into(m).font_size)
            .unwrap_or(16.0);

        let scale = font_size / font_metrics.units_per_em as f32;

        // Pre-calculate common values to avoid repeated computations
        let ascent_scaled = font_metrics.ascent as f32 * scale;
        let descent_scaled = font_metrics.descent as f32 * scale;
        let x_height_scaled = font_metrics.x_height.unwrap_or(500) as f32 * scale;

        // Calculate all baselines with optimized operations
        let ideographic = font_metrics
            .ideographic_baseline
            .map(|b| b as f32 * scale)
            .unwrap_or(-descent_scaled);

        let hanging = font_metrics
            .hanging_baseline
            .map(|b| b as f32 * scale)
            .unwrap_or(ascent_scaled * 0.8);

        let mathematical = font_metrics
            .mathematical_baseline
            .map(|b| b as f32 * scale)
            .unwrap_or(x_height_scaled * 0.5);

        // Central and middle calculations optimized
        let central_middle = x_height_scaled * 0.5;

        Ok(BaselineInfo {
            alphabetic: 0.0, // Baseline reference (no allocation)
            ideographic,
            hanging,
            mathematical,
            central: central_middle,
            middle: central_middle,
            text_top: ascent_scaled,
            text_bottom: -descent_scaled,
        })
    }

    /// Fast baseline calculation for single CSS baseline type (ultra-optimized)
    #[inline]
    pub fn calculate_single_baseline(
        &self,
        baseline_type: CSSBaselineType,
        attrs: &Attrs,
        font_metrics: &FontMetrics,
    ) -> MeasurementResult<f32> {
        let font_size = attrs
            .metrics_opt
            .map(|m| Into::<cosmyc_text::Metrics>::into(m).font_size)
            .unwrap_or(16.0);

        let scale = font_size / font_metrics.units_per_em as f32;

        let result = match baseline_type {
            CSSBaselineType::Alphabetic => 0.0,
            CSSBaselineType::Ideographic => font_metrics
                .ideographic_baseline
                .map(|b| b as f32 * scale)
                .unwrap_or(-font_metrics.descent as f32 * scale),
            CSSBaselineType::Hanging => font_metrics
                .hanging_baseline
                .map(|b| b as f32 * scale)
                .unwrap_or(font_metrics.ascent as f32 * scale * 0.8),
            CSSBaselineType::Mathematical => font_metrics
                .mathematical_baseline
                .map(|b| b as f32 * scale)
                .unwrap_or(font_metrics.x_height.unwrap_or(500) as f32 * scale * 0.5),
            CSSBaselineType::Central => font_metrics.x_height.unwrap_or(500) as f32 * scale * 0.5,
            CSSBaselineType::Middle => font_metrics.x_height.unwrap_or(500) as f32 * scale * 0.5,
            CSSBaselineType::TextTop => font_metrics.ascent as f32 * scale,
            CSSBaselineType::TextBottom => -font_metrics.descent as f32 * scale,
        };

        Ok(result)
    }

    /// Validate baseline calculations against font metrics (debug/validation only)
    #[inline]
    pub fn validate_baselines(
        &self,
        baseline_info: &BaselineInfo,
        font_metrics: &FontMetrics,
        font_size: f32,
    ) -> bool {
        let scale = font_size / font_metrics.units_per_em as f32;
        let expected_text_top = font_metrics.ascent as f32 * scale;
        let expected_text_bottom = -font_metrics.descent as f32 * scale;

        // Basic sanity checks
        baseline_info.alphabetic == 0.0
            && baseline_info.text_top == expected_text_top
            && baseline_info.text_bottom == expected_text_bottom
            && baseline_info.text_top >= baseline_info.alphabetic
            && baseline_info.text_bottom <= baseline_info.alphabetic
    }
}

impl Default for BaselineCalculator {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// CSS baseline types for optimized baseline calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CSSBaselineType {
    Alphabetic,
    Ideographic,
    Hanging,
    Mathematical,
    Central,
    Middle,
    TextTop,
    TextBottom,
}
