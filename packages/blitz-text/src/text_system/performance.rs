//! Performance monitoring for the unified text system
//!
//! This module handles performance tracking, statistics collection,
//! and system health monitoring for the text system.

use std::time::Instant;

/// System performance monitor
#[derive(Debug)]
pub struct SystemPerformanceMonitor {
    measurement_times: parking_lot::Mutex<Vec<std::time::Duration>>,
    preparation_times: parking_lot::Mutex<Vec<std::time::Duration>>,
    render_times: parking_lot::Mutex<Vec<std::time::Duration>>,
    start_time: Instant,
}

impl SystemPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            measurement_times: parking_lot::Mutex::new(Vec::new()),
            preparation_times: parking_lot::Mutex::new(Vec::new()),
            render_times: parking_lot::Mutex::new(Vec::new()),
            start_time: Instant::now(),
        }
    }

    pub fn record_measurement_time(&self, duration: std::time::Duration) {
        let mut times = self.measurement_times.lock();
        times.push(duration);

        // Keep only recent measurements (last 1000)
        if times.len() > 1000 {
            let excess = times.len() - 1000;
            times.drain(0..excess);
        }
    }

    pub fn record_preparation_time(&self, duration: std::time::Duration) {
        let mut times = self.preparation_times.lock();
        times.push(duration);

        if times.len() > 1000 {
            let excess = times.len() - 1000;
            times.drain(0..excess);
        }
    }

    pub fn record_render_time(&self, duration: std::time::Duration) {
        let mut times = self.render_times.lock();
        times.push(duration);

        if times.len() > 1000 {
            let excess = times.len() - 1000;
            times.drain(0..excess);
        }
    }

    pub fn get_stats(&self) -> SystemPerformanceStats {
        let measurement_times = self.measurement_times.lock();
        let preparation_times = self.preparation_times.lock();
        let render_times = self.render_times.lock();

        SystemPerformanceStats {
            avg_measurement_time: Self::calculate_average(&measurement_times),
            avg_preparation_time: Self::calculate_average(&preparation_times),
            avg_render_time: Self::calculate_average(&render_times),
            total_operations: measurement_times.len()
                + preparation_times.len()
                + render_times.len(),
            monitoring_duration: self.start_time.elapsed(),
        }
    }

    fn calculate_average(times: &[std::time::Duration]) -> std::time::Duration {
        if times.is_empty() {
            std::time::Duration::ZERO
        } else {
            let total: std::time::Duration = times.iter().sum();
            total / times.len() as u32
        }
    }

    pub fn reset(&mut self) {
        self.measurement_times.lock().clear();
        self.preparation_times.lock().clear();
        self.render_times.lock().clear();
        self.start_time = Instant::now();
    }
}

/// System performance statistics
#[derive(Debug, Clone, Copy)]
pub struct SystemPerformanceStats {
    pub avg_measurement_time: std::time::Duration,
    pub avg_preparation_time: std::time::Duration,
    pub avg_render_time: std::time::Duration,
    pub total_operations: usize,
    pub monitoring_duration: std::time::Duration,
}

/// Comprehensive system statistics
#[derive(Debug, Clone)]
pub struct ComprehensiveStats {
    pub measurement_stats: crate::measurement::MeasurementStats,
    pub cosmyc_integration_stats: crate::cosmyc::IntegrationStats,
    pub gpu_render_stats: crate::gpu::GpuRenderStats,
    pub gpu_cache_stats: crate::gpu::cache::GpuCacheStats,
    pub atlas_stats: crate::gpu::text_atlas::AtlasStats,
    pub viewport_stats: crate::gpu::viewport::ViewportStats,
    pub performance_stats: SystemPerformanceStats,
    pub system_health: f64,
    pub stats_duration: std::time::Duration,
}

/// System optimization results
#[derive(Debug, Clone)]
pub struct SystemOptimizationResult {
    pub cosmyc_optimization: crate::cosmyc::IntegrationOptimizationResult,
    pub cache_optimization: crate::gpu::cache::CacheOptimizationResult,
    pub atlas_optimization: crate::gpu::text_atlas::OptimizationResult,
    pub total_optimization_time: std::time::Duration,
    pub memory_saved: usize,
}
