//! Performance analytics and monitoring for viewport operations
//!
//! This module handles performance statistics collection, frequency calculation,
//! and optimization detection for viewport operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use super::types::ViewportStats;

/// Performance analytics manager for viewport operations
pub struct PerformanceAnalytics {
    /// Performance statistics (atomic for thread safety)
    resolution_updates: AtomicU64,
    total_resolution_changes: AtomicU64,

    /// Performance tracking
    last_update_time: Mutex<Instant>,
    update_frequency_hz: Mutex<f64>,

    /// Statistics
    stats_reset_time: Instant,
}

impl PerformanceAnalytics {
    /// Create new performance analytics manager
    pub fn new() -> Self {
        Self {
            resolution_updates: AtomicU64::new(0),
            total_resolution_changes: AtomicU64::new(0),
            last_update_time: Mutex::new(Instant::now()),
            update_frequency_hz: Mutex::new(0.0),
            stats_reset_time: Instant::now(),
        }
    }

    /// Record a resolution update
    pub fn record_resolution_update(&self, resolution_changed: bool) {
        self.resolution_updates.fetch_add(1, Ordering::Relaxed);

        if resolution_changed {
            self.total_resolution_changes
                .fetch_add(1, Ordering::Relaxed);
        }

        // Update frequency calculation
        let current_time = Instant::now();
        self.calculate_update_frequency(current_time);
        *self.last_update_time.lock() = current_time;
    }

    /// Calculate update frequency based on recent updates
    fn calculate_update_frequency(&self, current_time: Instant) {
        let last_update = *self.last_update_time.lock();
        let time_diff = current_time.duration_since(last_update);

        if time_diff > Duration::from_millis(1) {
            let current_frequency = 1.0 / time_diff.as_secs_f64();

            // Exponential moving average for smoothing
            let mut frequency = self.update_frequency_hz.lock();
            const ALPHA: f64 = 0.1; // Smoothing factor
            *frequency = ALPHA * current_frequency + (1.0 - ALPHA) * *frequency;
        }
    }

    /// Get comprehensive viewport statistics
    pub fn get_stats(&self, current_resolution: glyphon::Resolution) -> ViewportStats {
        ViewportStats {
            resolution_updates: self.resolution_updates.load(Ordering::Relaxed),
            total_resolution_changes: self.total_resolution_changes.load(Ordering::Relaxed),
            current_resolution,
            update_frequency_hz: *self.update_frequency_hz.lock(),
            stats_duration: self.stats_reset_time.elapsed(),
        }
    }

    /// Check if viewport should be optimized
    pub fn should_optimize(&self) -> bool {
        let resolution_updates = self.resolution_updates.load(Ordering::Relaxed);
        let total_changes = self.total_resolution_changes.load(Ordering::Relaxed);
        let frequency = *self.update_frequency_hz.lock();

        // Optimize if there are frequent resolution changes
        frequency > 30.0 || // More than 30 updates per second
        total_changes > 1000 || // Too many total changes
        (resolution_updates > 0 && (total_changes as f64 / resolution_updates as f64) > 0.5)
        // High change ratio
    }

    /// Get optimization recommendations
    pub fn get_optimization_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        let resolution_updates = self.resolution_updates.load(Ordering::Relaxed);
        let total_changes = self.total_resolution_changes.load(Ordering::Relaxed);
        let frequency = *self.update_frequency_hz.lock();

        if frequency > 60.0 {
            recommendations.push(OptimizationRecommendation {
                priority: OptimizationPriority::High,
                category: OptimizationCategory::UpdateFrequency,
                description: format!(
                    "Very high update frequency ({:.1} Hz). Consider throttling updates or using frame-based updates.",
                    frequency
                ),
                estimated_impact: 0.8,
            });
        } else if frequency > 30.0 {
            recommendations.push(OptimizationRecommendation {
                priority: OptimizationPriority::Medium,
                category: OptimizationCategory::UpdateFrequency,
                description: format!(
                    "High update frequency ({:.1} Hz). Consider reducing update rate.",
                    frequency
                ),
                estimated_impact: 0.5,
            });
        }

        if resolution_updates > 0 {
            let change_ratio = total_changes as f64 / resolution_updates as f64;
            if change_ratio > 0.7 {
                recommendations.push(OptimizationRecommendation {
                    priority: OptimizationPriority::High,
                    category: OptimizationCategory::ResolutionStability,
                    description: format!(
                        "Very high resolution change ratio ({:.1}%). Consider resolution caching or debouncing.",
                        change_ratio * 100.0
                    ),
                    estimated_impact: 0.7,
                });
            } else if change_ratio > 0.3 {
                recommendations.push(OptimizationRecommendation {
                    priority: OptimizationPriority::Medium,
                    category: OptimizationCategory::ResolutionStability,
                    description: format!(
                        "High resolution change ratio ({:.1}%). Consider resolution validation.",
                        change_ratio * 100.0
                    ),
                    estimated_impact: 0.4,
                });
            }
        }

        if total_changes > 10000 {
            recommendations.push(OptimizationRecommendation {
                priority: OptimizationPriority::Low,
                category: OptimizationCategory::HistoryManagement,
                description:
                    "Very high total resolution changes. Consider clearing history periodically."
                        .to_string(),
                estimated_impact: 0.2,
            });
        }

        recommendations
    }

    /// Reset all performance statistics
    pub fn reset_stats(&mut self) {
        self.resolution_updates.store(0, Ordering::Relaxed);
        self.total_resolution_changes.store(0, Ordering::Relaxed);
        *self.update_frequency_hz.lock() = 0.0;
        self.stats_reset_time = Instant::now();
    }

    /// Get performance score (0.0 to 1.0, higher is better)
    pub fn get_performance_score(&self, current_resolution: glyphon::Resolution) -> f64 {
        let stats = self.get_stats(current_resolution);

        // Calculate individual scores
        let stability_score = stats.stability_score();
        let efficiency_score = stats.efficiency_score();

        // Frequency penalty (too high frequency is bad)
        let frequency_score = if stats.update_frequency_hz > 60.0 {
            0.2
        } else if stats.update_frequency_hz > 30.0 {
            0.6
        } else {
            1.0
        };

        // Weighted average
        stability_score * 0.4 + efficiency_score * 0.4 + frequency_score * 0.2
    }
}

impl Default for PerformanceAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimization recommendation
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    pub priority: OptimizationPriority,
    pub category: OptimizationCategory,
    pub description: String,
    pub estimated_impact: f64, // 0.0 to 1.0
}

/// Priority level for optimization recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Category of optimization recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationCategory {
    UpdateFrequency,
    ResolutionStability,
    MemoryUsage,
    HistoryManagement,
    Configuration,
}
