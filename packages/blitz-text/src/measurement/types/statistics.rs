//! Statistics type definitions for measurement system
//!
//! This module contains types for tracking and reporting performance statistics
//! of the text measurement system, including cache hit rates and metrics.

use std::sync::atomic::{AtomicU64, AtomicUsize};

/// Comprehensive measurement statistics for performance monitoring
#[derive(Debug)]
pub struct MeasurementStatsInner {
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub total_measurements: AtomicU64,
    pub font_metrics_cache_hits: AtomicU64,
    pub font_metrics_cache_misses: AtomicU64,
    pub baseline_cache_hits: AtomicU64,
    pub baseline_cache_misses: AtomicU64,
    pub evictions: AtomicU64,
    pub current_cache_size: AtomicUsize,
}

impl Default for MeasurementStatsInner {
    fn default() -> Self {
        Self::new()
    }
}

impl MeasurementStatsInner {
    /// Create new measurement statistics inner structure
    pub fn new() -> Self {
        Self {
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_measurements: AtomicU64::new(0),
            font_metrics_cache_hits: AtomicU64::new(0),
            font_metrics_cache_misses: AtomicU64::new(0),
            baseline_cache_hits: AtomicU64::new(0),
            baseline_cache_misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            current_cache_size: AtomicUsize::new(0),
        }
    }
}

/// Public statistics view for measurement system performance
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_measurements: u64,
    pub font_metrics_cache_hits: u64,
    pub font_metrics_cache_misses: u64,
    pub baseline_cache_hits: u64,
    pub baseline_cache_misses: u64,
    pub evictions: u64,
    pub current_cache_size: usize,
    pub hit_rate: f32,
    pub font_metrics_hit_rate: f32,
    pub baseline_hit_rate: f32,
}
