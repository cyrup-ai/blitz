//! Optimization and prediction functionality
//!
//! This module provides optimization functionality for the text atlas system,
//! including growth prediction and performance tuning.

use std::sync::atomic::Ordering;
use std::time::Instant;

use super::core::EnhancedTextAtlas;
use super::types::{GrowthPrediction, OptimizationResult};

impl EnhancedTextAtlas {
    /// Predict if atlas growth will be needed for upcoming glyphs
    pub fn predict_growth_needed(&self, estimated_new_glyphs: u32) -> GrowthPrediction {
        let current_memory = self.estimated_memory_usage.load(Ordering::Relaxed);
        let current_glyphs = self
            .glyph_allocations
            .load(Ordering::Relaxed)
            .saturating_sub(self.glyph_deallocations.load(Ordering::Relaxed));

        // Estimate memory per glyph based on current usage
        let avg_memory_per_glyph = if current_glyphs > 0 {
            current_memory as f64 / current_glyphs as f64
        } else {
            256.0 // Conservative estimate for average glyph size
        };

        let estimated_additional_memory =
            (estimated_new_glyphs as f64 * avg_memory_per_glyph) as usize;
        let predicted_total_memory = current_memory + estimated_additional_memory;

        // Check against current atlas capacity
        let color_size = self.color_atlas_size.load(Ordering::Relaxed);
        let mask_size = self.mask_atlas_size.load(Ordering::Relaxed);
        let current_capacity = ((color_size * color_size * 4) + (mask_size * mask_size)) as usize;

        GrowthPrediction {
            growth_needed: predicted_total_memory > current_capacity,
            estimated_additional_memory,
            predicted_total_memory,
            current_capacity,
            confidence: if current_glyphs > 10 { 0.8 } else { 0.5 },
        }
    }

    /// Optimize atlas packing and memory usage
    pub fn optimize_packing(&mut self) -> OptimizationResult {
        let start_time = Instant::now();
        let memory_before = self.estimated_memory_usage.load(Ordering::Relaxed);

        // Perform trim to remove unused glyphs
        self.trim_enhanced();

        let memory_after = self.estimated_memory_usage.load(Ordering::Relaxed);
        let memory_saved = memory_before.saturating_sub(memory_after);

        // Update last optimization time
        *self.last_optimization_time.lock() = start_time;

        OptimizationResult {
            memory_saved,
            optimization_time: start_time.elapsed(),
            glyphs_removed: 0, // We don't have direct access to this from glyphon
            fragmentation_reduced: memory_saved as f64 / memory_before.max(1) as f64,
        }
    }

    /// Check if optimization is recommended based on usage patterns
    pub fn should_optimize(&self) -> bool {
        let last_optimization = *self.last_optimization_time.lock();
        let time_since_optimization = last_optimization.elapsed();

        // Optimization heuristics
        let frequent_trims = self.trim_operations.load(Ordering::Relaxed) > 10;
        let many_deallocations = self.glyph_deallocations.load(Ordering::Relaxed) > 100;
        let time_threshold = time_since_optimization > std::time::Duration::from_secs(60);
        let memory_pressure = {
            let current = self.estimated_memory_usage.load(Ordering::Relaxed);
            let peak = self.peak_memory_usage.load(Ordering::Relaxed);
            peak > 0 && (current as f64 / peak as f64) < 0.7 // Less than 70% of peak usage
        };

        (frequent_trims || many_deallocations || memory_pressure) && time_threshold
    }
}
