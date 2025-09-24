//! System management operations for optimization and configuration
//!
//! This module handles system-wide operations including statistics,
//! health monitoring, optimization, and configuration management.

use std::time::Instant;

use super::super::config::UnifiedTextConfig;
use super::super::performance::{ComprehensiveStats, SystemOptimizationResult};
use super::system::UnifiedTextSystem;
use crate::gpu::cache::{CacheOptimizationResult, GpuCacheConfig};
use crate::gpu::GpuRenderConfig;

/// Convert GpuRenderConfig to GpuCacheConfig
fn convert_render_config_to_cache_config(render_config: &GpuRenderConfig) -> GpuCacheConfig {
    GpuCacheConfig {
        max_entries: 10000, // Reasonable default based on render config
        max_memory_mb: (render_config.max_memory_usage_mb as u32).max(256),
        compression_enabled: true, // Default to enabled for better memory usage
    }
}

impl UnifiedTextSystem {
    /// Get comprehensive system statistics
    pub fn get_comprehensive_stats(&self) -> ComprehensiveStats {
        ComprehensiveStats {
            measurement_stats: self.text_measurer.get_stats(),
            cosmyc_integration_stats: self.cosmyc_integration.get_stats(),
            gpu_render_stats: self.text_renderer.get_stats(),
            gpu_cache_stats: self.gpu_cache.get_stats(),
            atlas_stats: self.text_atlas.get_stats(),
            viewport_stats: self.viewport.get_stats(),
            performance_stats: self.performance_monitor.get_stats(),
            system_health: self.calculate_system_health(),
            stats_duration: self.stats_start_time.elapsed(),
        }
    }

    /// Calculate overall system health score (0.0 to 1.0)
    fn calculate_system_health(&self) -> f64 {
        let cosmyc_health = self.cosmyc_integration.health_score();
        let gpu_health = {
            let gpu_stats = self.text_renderer.get_stats();
            gpu_stats.efficiency_score()
        };
        let cache_health = {
            let cache_stats = self.gpu_cache.get_stats();
            // Calculate cache efficiency from hits and misses
            let total_requests = cache_stats.hits + cache_stats.misses;
            if total_requests > 0 {
                cache_stats.hits as f64 / total_requests as f64
            } else {
                1.0 // Perfect efficiency when no requests
            }
        };
        let atlas_health = {
            // Atlas health calculation - simplified since we don't have stats method
            0.9 // Default to good health
        };

        // Weighted average of component health scores
        (cosmyc_health * 0.3) + (gpu_health * 0.3) + (cache_health * 0.2) + (atlas_health * 0.2)
    }

    /// Optimize all system components
    pub fn optimize_system(&mut self) -> SystemOptimizationResult {
        let start_time = Instant::now();

        // Optimize cosmyc-text integration
        let cosmyc_result = self.cosmyc_integration.optimize_all();

        // Optimize GPU cache
        let cache_result = match self.gpu_cache.optimize_cache() {
            Ok(_) => CacheOptimizationResult {
                entries_removed: 0, // GPU cache doesn't provide detailed stats
                memory_freed: 0,
                fragmentation_reduced: 0.0,
            },
            Err(_) => CacheOptimizationResult::default(),
        };

        // Optimize text atlas
        let atlas_result = self.text_atlas.optimize_packing();

        // Reset performance monitor for clean slate
        self.performance_monitor.reset();

        SystemOptimizationResult {
            cosmyc_optimization: cosmyc_result,
            cache_optimization: cache_result.clone(),
            atlas_optimization: atlas_result,
            total_optimization_time: start_time.elapsed(),
            memory_saved: cache_result.memory_freed + atlas_result.memory_saved,
        }
    }

    /// Check if system optimization is recommended
    pub fn should_optimize(&self) -> bool {
        self.cosmyc_integration.should_optimize()
            || self.gpu_cache.should_optimize()
            || self.text_atlas.should_optimize()
    }

    /// Update system configuration
    pub fn update_config(&mut self, config: UnifiedTextConfig) {
        self.config = config;

        // Apply configuration to components
        if let Some(gpu_config) = &self.config.gpu_config {
            self.text_renderer.set_config(gpu_config.clone());
            self.text_atlas.set_config(gpu_config.clone());
            self.viewport.set_config(gpu_config.clone());
            self.gpu_cache
                .set_config(convert_render_config_to_cache_config(gpu_config));
        }
    }

    /// Reset all system statistics
    pub fn reset_all_stats(&mut self) {
        self.cosmyc_integration.reset_all_stats();
        self.text_renderer.reset_stats();
        self.text_atlas.reset_stats();
        self.viewport.reset_stats();
        self.gpu_cache.reset_stats();
        self.performance_monitor.reset();
        self.stats_start_time = Instant::now();
    }
}
