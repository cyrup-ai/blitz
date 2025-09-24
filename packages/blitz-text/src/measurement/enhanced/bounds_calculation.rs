//! Comprehensive text bounds calculations with zero allocation optimization

use crate::measurement::types::{InkBounds, LineMeasurement, LogicalBounds, TextBounds};

/// Bounds calculator for comprehensive text bounds computation with optimized algorithms
pub struct BoundsCalculator;

impl BoundsCalculator {
    /// Create new bounds calculator (zero allocation)
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Calculate comprehensive bounds from line measurements with optimized iteration
    /// Computes both logical bounds (based on line dimensions) and ink bounds (actual character positions)
    #[inline]
    pub fn calculate_comprehensive_bounds(
        &self,
        line_measurements: &[LineMeasurement],
    ) -> TextBounds {
        // Handle empty case efficiently
        if line_measurements.is_empty() {
            return TextBounds::default();
        }

        // Initialize bounds with optimized starting values
        let mut x_min = f32::INFINITY;
        let mut y_min = f32::INFINITY;
        let mut x_max = f32::NEG_INFINITY;
        let mut y_max = f32::NEG_INFINITY;

        let mut ink_x_min = f32::INFINITY;
        let mut ink_y_min = f32::INFINITY;
        let mut ink_x_max = f32::NEG_INFINITY;
        let mut ink_y_max = f32::NEG_INFINITY;

        // Process line measurements with cache-friendly iteration
        for (line_index, line) in line_measurements.iter().enumerate() {
            let line_y = line_index as f32 * line.height;

            // Logical bounds calculation (based on line dimensions)
            x_min = x_min.min(0.0);
            x_max = x_max.max(line.width);
            y_min = y_min.min(line_y);
            y_max = y_max.max(line_y + line.height);

            // Ink bounds calculation (based on actual character positions)
            for char_pos in &line.character_positions {
                let char_x_max = char_pos.x + char_pos.width;
                let char_y_max = char_pos.y + char_pos.height;

                ink_x_min = ink_x_min.min(char_pos.x);
                ink_x_max = ink_x_max.max(char_x_max);
                ink_y_min = ink_y_min.min(char_pos.y);
                ink_y_max = ink_y_max.max(char_y_max);
            }
        }

        // Finalize bounds with validation (handle edge cases)
        let (final_x_min, final_y_min, final_x_max, final_y_max) = if x_min.is_infinite() {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            (x_min, y_min, x_max, y_max)
        };

        let (final_ink_x_min, final_ink_y_min, final_ink_x_max, final_ink_y_max) =
            if ink_x_min.is_infinite() {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                (ink_x_min, ink_y_min, ink_x_max, ink_y_max)
            };

        TextBounds {
            x_min: final_x_min,
            y_min: final_y_min,
            x_max: final_x_max,
            y_max: final_y_max,
            ink_bounds: InkBounds {
                x_min: final_ink_x_min,
                y_min: final_ink_y_min,
                x_max: final_ink_x_max,
                y_max: final_ink_y_max,
            },
            logical_bounds: LogicalBounds {
                x_min: final_x_min,
                y_min: final_y_min,
                x_max: final_x_max,
                y_max: final_y_max,
            },
        }
    }

    /// Fast bounds estimation for performance-critical paths
    #[inline]
    pub fn estimate_bounds_fast(&self, line_measurements: &[LineMeasurement]) -> (f32, f32) {
        let total_width = line_measurements
            .iter()
            .map(|line| line.width)
            .fold(0.0f32, f32::max);
        let total_height = line_measurements
            .iter()
            .map(|line| line.height)
            .sum::<f32>();
        (total_width, total_height)
    }

    /// Check if bounds calculation is needed based on content
    #[inline]
    pub fn bounds_needed(&self, line_measurements: &[LineMeasurement]) -> bool {
        !line_measurements.is_empty()
    }

    /// Validate bounds for correctness (debug/testing only)
    #[inline]
    pub fn validate_bounds(&self, bounds: &TextBounds) -> bool {
        bounds.x_min <= bounds.x_max
            && bounds.y_min <= bounds.y_max
            && bounds.ink_bounds.x_min <= bounds.ink_bounds.x_max
            && bounds.ink_bounds.y_min <= bounds.ink_bounds.y_max
            && bounds.logical_bounds.x_min <= bounds.logical_bounds.x_max
            && bounds.logical_bounds.y_min <= bounds.logical_bounds.y_max
    }
}

impl Default for BoundsCalculator {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
