//! Fast metrics computation with SIMD optimization

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::shaping::types::ShapedRun;

/// Statistics for metrics calculation
static METRICS_CALCULATIONS: AtomicUsize = AtomicUsize::new(0);
static SIMD_OPTIMIZATIONS: AtomicUsize = AtomicUsize::new(0);

/// Fast metrics calculator with SIMD acceleration
pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Calculate metrics (compile-time optimized)
    pub fn calculate_metrics_fast(runs: &[ShapedRun]) -> (f32, f32, f32, usize) {
        METRICS_CALCULATIONS.fetch_add(1, Ordering::Relaxed);

        if runs.is_empty() {
            return (0.0, 0.0, 0.0, 0);
        }

        // Use SIMD optimization for large runs
        if runs.len() >= 4 {
            SIMD_OPTIMIZATIONS.fetch_add(1, Ordering::Relaxed);
            return Self::calculate_metrics_simd(runs);
        }

        // Fallback to scalar calculation for small runs
        Self::calculate_metrics_scalar(runs)
    }

    /// SIMD-accelerated metrics calculation for large run arrays
    #[cfg(target_arch = "x86_64")]
    fn calculate_metrics_simd(runs: &[ShapedRun]) -> (f32, f32, f32, usize) {
        unsafe {
            let mut total_width = 0.0f32;
            let mut max_height = 0.0f32;
            let mut baseline = 0.0f32;

            // Process runs in chunks of 4 for SIMD optimization
            let chunks = runs.chunks_exact(4);
            let remainder = chunks.remainder();

            for chunk in chunks {
                // Load 4 widths into SIMD register
                let widths = _mm_set_ps(
                    chunk[3].width,
                    chunk[2].width,
                    chunk[1].width,
                    chunk[0].width,
                );
                let heights = _mm_set_ps(
                    chunk[3].height,
                    chunk[2].height,
                    chunk[1].height,
                    chunk[0].height,
                );
                let ascents = _mm_set_ps(
                    chunk[3].ascent,
                    chunk[2].ascent,
                    chunk[1].ascent,
                    chunk[0].ascent,
                );

                // Horizontal sum for total width
                let width_sum = Self::horizontal_sum_f32(widths);
                total_width += width_sum;

                // Maximum height calculation
                let max_height_vec = _mm_max_ps(heights, _mm_set1_ps(max_height));
                max_height = Self::horizontal_max_f32(max_height_vec);

                // Maximum baseline calculation
                let max_baseline_vec = _mm_max_ps(ascents, _mm_set1_ps(baseline));
                baseline = Self::horizontal_max_f32(max_baseline_vec);
            }

            // Process remaining runs
            for run in remainder {
                total_width += run.width;
                max_height = max_height.max(run.height);
                baseline = baseline.max(run.ascent);
            }

            let line_count = 1; // Simplified line counting
            (total_width, max_height, baseline, line_count)
        }
    }

    /// Fallback scalar calculation
    #[cfg(not(target_arch = "x86_64"))]
    fn calculate_metrics_simd(runs: &[ShapedRun]) -> (f32, f32, f32, usize) {
        Self::calculate_metrics_scalar(runs)
    }

    /// Scalar metrics calculation
    fn calculate_metrics_scalar(runs: &[ShapedRun]) -> (f32, f32, f32, usize) {
        let mut total_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;
        let mut baseline: f32 = 0.0;

        for run in runs {
            total_width += run.width;
            max_height = max_height.max(run.height);
            baseline = baseline.max(run.ascent);
        }

        let line_count = 1; // Simplified line counting
        (total_width, max_height, baseline, line_count)
    }

    /// SIMD helper: horizontal sum of 4 f32 values
    #[cfg(target_arch = "x86_64")]
    #[inline]
    unsafe fn horizontal_sum_f32(v: __m128) -> f32 {
        let shuf = _mm_movehdup_ps(v); // duplicate elements 1,3
        let sums = _mm_add_ps(v, shuf); // add elements 0+1, 1+1, 2+3, 3+3
        let shuf = _mm_movehl_ps(shuf, sums); // high two elements
        let sums = _mm_add_ss(sums, shuf); // add low elements
        _mm_cvtss_f32(sums)
    }

    /// SIMD helper: horizontal maximum of 4 f32 values
    #[cfg(target_arch = "x86_64")]
    #[inline]
    unsafe fn horizontal_max_f32(v: __m128) -> f32 {
        let shuf = _mm_movehdup_ps(v);
        let maxs = _mm_max_ps(v, shuf);
        let shuf = _mm_movehl_ps(shuf, maxs);
        let maxs = _mm_max_ss(maxs, shuf);
        _mm_cvtss_f32(maxs)
    }

    /// Calculate line metrics for multi-line text
    pub fn calculate_line_metrics(runs: &[ShapedRun], line_height: f32) -> LineMetrics {
        let (total_width, height, baseline, _) = Self::calculate_metrics_fast(runs);

        LineMetrics {
            total_width,
            total_height: height.max(line_height),
            baseline,
            line_height,
            ascent: baseline,
            descent: height - baseline,
        }
    }

    /// Calculate bounding box for shaped runs
    pub fn calculate_bounding_box(runs: &[ShapedRun]) -> BoundingBox {
        if runs.is_empty() {
            return BoundingBox::default();
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut current_x = 0.0;

        for run in runs {
            // Calculate run bounds
            let run_min_x = current_x;
            let run_max_x = current_x + run.width;
            let run_min_y = -run.ascent;
            let run_max_y = run.descent;

            min_x = min_x.min(run_min_x);
            max_x = max_x.max(run_max_x);
            min_y = min_y.min(run_min_y);
            max_y = max_y.max(run_max_y);

            current_x += run.width;
        }

        BoundingBox {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// Get metrics calculation statistics
    pub fn stats() -> MetricsStats {
        let calculations = METRICS_CALCULATIONS.load(Ordering::Relaxed);
        let simd_optimizations = SIMD_OPTIMIZATIONS.load(Ordering::Relaxed);

        MetricsStats {
            calculations,
            simd_optimizations,
            simd_usage_rate: if calculations > 0 {
                simd_optimizations as f64 / calculations as f64
            } else {
                0.0
            },
        }
    }
}

/// Line metrics information
#[derive(Debug, Clone)]
pub struct LineMetrics {
    pub total_width: f32,
    pub total_height: f32,
    pub baseline: f32,
    pub line_height: f32,
    pub ascent: f32,
    pub descent: f32,
}

/// Bounding box information
#[derive(Debug, Clone, Default)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Metrics calculation statistics
#[derive(Debug, Clone)]
pub struct MetricsStats {
    pub calculations: usize,
    pub simd_optimizations: usize,
    pub simd_usage_rate: f64,
}
