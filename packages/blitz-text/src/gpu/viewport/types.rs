//! Core types and data structures for viewport management
//!
//! This module contains all the data structures used for viewport statistics,
//! resolution tracking, and analysis results.

use std::time::{Duration, Instant};

use glyphon::Resolution;

/// Viewport performance statistics
#[derive(Debug, Clone, Copy)]
pub struct ViewportStats {
    pub resolution_updates: u64,
    pub total_resolution_changes: u64,
    pub current_resolution: Resolution,
    pub update_frequency_hz: f64,
    pub stats_duration: Duration,
}

impl ViewportStats {
    /// Get stability score (0.0 to 1.0, higher is more stable)
    pub fn stability_score(&self) -> f64 {
        if self.resolution_updates == 0 {
            return 1.0;
        }

        let change_ratio = self.total_resolution_changes as f64 / self.resolution_updates as f64;
        1.0 / (1.0 + change_ratio)
    }

    /// Get efficiency score based on update patterns
    pub fn efficiency_score(&self) -> f64 {
        // Low frequency is more efficient (fewer unnecessary updates)
        let frequency_efficiency = if self.update_frequency_hz > 0.0 {
            1.0 / (1.0 + self.update_frequency_hz / 60.0) // 60 FPS as baseline
        } else {
            1.0
        };

        // High stability is more efficient
        let stability_efficiency = self.stability_score();

        (frequency_efficiency + stability_efficiency) / 2.0
    }
}

/// Resolution change event tracking
#[derive(Debug, Clone, Copy)]
pub struct ResolutionEvent {
    pub timestamp: Instant,
    pub old_resolution: Resolution,
    pub new_resolution: Resolution,
    pub duration_since_last_change: Duration,
}

impl ResolutionEvent {
    /// Get the resolution change magnitude (0.0 to 1.0+)
    pub fn change_magnitude(&self) -> f64 {
        let old_pixels = (self.old_resolution.width as u64) * (self.old_resolution.height as u64);
        let new_pixels = (self.new_resolution.width as u64) * (self.new_resolution.height as u64);

        if old_pixels == 0 {
            return 1.0;
        }

        (new_pixels as f64 - old_pixels as f64).abs() / old_pixels as f64
    }

    /// Check if this is a significant resolution change
    pub fn is_significant_change(&self) -> bool {
        self.change_magnitude() > 0.1 // 10% change threshold
    }
}

/// Resolution usage pattern analysis
#[derive(Debug, Clone, Copy)]
pub struct ResolutionAnalysis {
    pub most_common_resolution: Resolution,
    pub avg_resolution: Resolution,
    pub resolution_stability: f64, // 0.0 to 1.0
    pub change_frequency_hz: f64,
    pub aspect_ratio_consistency: f64, // 0.0 to 1.0
}

/// Optimal resolution prediction
#[derive(Debug, Clone)]
pub struct OptimalResolutionPrediction {
    pub predicted_resolution: Resolution,
    pub confidence: f64, // 0.0 to 1.0
    pub reasoning: String,
}
