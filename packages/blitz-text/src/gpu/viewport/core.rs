//! Core Enhanced Viewport implementation
//!
//! This module contains the main EnhancedViewport struct and its core functionality,
//! integrating resolution management and performance analytics.

use std::time::Instant;

// Re-export glyphon types for convenience
pub use glyphon::{Cache, Resolution, Viewport};
use parking_lot::Mutex;
use wgpu::{Device, Queue};

use super::performance_analytics::PerformanceAnalytics;
use super::resolution_manager::ResolutionManager;
use super::types::{
    OptimalResolutionPrediction, ResolutionAnalysis, ResolutionEvent, ViewportStats,
};
use crate::gpu::{GpuRenderConfig, GpuTextResult};

/// Enhanced Viewport with comprehensive performance monitoring and optimization
pub struct EnhancedViewport {
    /// Inner glyphon Viewport (None in headless mode)
    inner: Option<Viewport>,

    /// Resolution management
    resolution_manager: Mutex<ResolutionManager>,

    /// Performance analytics
    performance_analytics: PerformanceAnalytics,

    /// Configuration
    config: GpuRenderConfig,
}

impl EnhancedViewport {
    /// Create a headless viewport for DOM operations without GPU context
    pub fn headless() -> Self {
        // No GPU context in headless mode
        let inner = None;

        Self {
            inner,
            resolution_manager: Mutex::new(ResolutionManager::new()),
            performance_analytics: PerformanceAnalytics::new(),
            config: GpuRenderConfig::default(),
        }
    }

    /// Create a new enhanced viewport
    pub fn new(device: &Device, cache: &Cache) -> Self {
        let inner = Some(Viewport::new(device, cache));

        Self {
            inner,
            resolution_manager: Mutex::new(ResolutionManager::new()),
            performance_analytics: PerformanceAnalytics::new(),
            config: GpuRenderConfig::default(),
        }
    }

    /// Create a new enhanced viewport with custom configuration
    pub fn with_config(device: &Device, cache: &Cache, config: GpuRenderConfig) -> Self {
        let mut viewport = Self::new(device, cache);
        viewport.config = config;
        viewport
    }

    /// Update viewport with enhanced resolution tracking
    pub fn update_enhanced(&mut self, queue: &Queue, resolution: Resolution) -> GpuTextResult<()> {
        let update_time = Instant::now();

        // Update resolution through manager
        let resolution_changed = {
            let mut manager = self.resolution_manager.lock();
            manager.update_resolution(resolution, update_time)?
        };

        // Call inner update method (skip in headless mode)
        if let Some(ref mut inner) = self.inner {
            inner.update(queue, resolution);
        }

        // Record performance metrics
        self.performance_analytics
            .record_resolution_update(resolution_changed);

        Ok(())
    }

    /// Get the current resolution
    pub fn resolution(&self) -> Resolution {
        self.inner
            .as_ref()
            .map(|inner| inner.resolution())
            .unwrap_or_else(|| Resolution {
                width: 0,
                height: 0,
            })
    }

    /// Get the current resolution with thread safety
    pub fn resolution_safe(&self) -> Resolution {
        self.resolution_manager.lock().current_resolution()
    }

    /// Get comprehensive viewport statistics
    pub fn get_stats(&self) -> ViewportStats {
        let current_resolution = self.resolution_safe();
        self.performance_analytics.get_stats(current_resolution)
    }

    /// Get resolution change history
    pub fn get_resolution_history(&self) -> Vec<ResolutionEvent> {
        self.resolution_manager
            .lock()
            .get_resolution_history()
            .to_vec()
    }

    /// Analyze resolution patterns
    pub fn analyze_resolution_patterns(&self) -> ResolutionAnalysis {
        self.resolution_manager.lock().analyze_resolution_patterns()
    }

    /// Predict optimal resolution based on usage patterns
    pub fn predict_optimal_resolution(&self) -> OptimalResolutionPrediction {
        self.resolution_manager.lock().predict_optimal_resolution()
    }

    /// Check if viewport should be optimized
    pub fn should_optimize(&self) -> bool {
        self.performance_analytics.should_optimize()
    }

    /// Get optimization recommendations
    pub fn get_optimization_recommendations(
        &self,
    ) -> Vec<super::performance_analytics::OptimizationRecommendation> {
        self.performance_analytics
            .get_optimization_recommendations()
    }

    /// Get performance score (0.0 to 1.0, higher is better)
    pub fn get_performance_score(&self) -> f64 {
        let current_resolution = self.resolution_safe();
        self.performance_analytics
            .get_performance_score(current_resolution)
    }

    /// Reset all performance statistics
    pub fn reset_stats(&mut self) {
        self.performance_analytics.reset_stats();
        self.resolution_manager.lock().clear_history();
    }

    /// Get the current configuration
    pub fn config(&self) -> &GpuRenderConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: GpuRenderConfig) {
        self.config = config;
    }

    /// Get reference to inner Viewport for advanced usage
    pub fn inner(&self) -> Option<&Viewport> {
        self.inner.as_ref()
    }

    /// Get mutable reference to inner Viewport for advanced usage
    pub fn inner_mut(&mut self) -> Option<&mut Viewport> {
        self.inner.as_mut()
    }
}
