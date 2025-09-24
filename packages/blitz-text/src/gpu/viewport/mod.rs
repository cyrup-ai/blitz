//! Enhanced Viewport module with comprehensive performance monitoring
//!
//! This module provides GPU viewport management with enhanced resolution tracking,
//! performance analytics, and intelligent optimization strategies.

pub mod core;
pub mod performance_analytics;
pub mod resolution_manager;
pub mod types;

// Re-export main types and functionality
pub use core::EnhancedViewport;

// Re-export glyphon types for convenience
pub use glyphon::{Cache, Resolution, Viewport};
pub use performance_analytics::{
    OptimizationCategory, OptimizationPriority, OptimizationRecommendation, PerformanceAnalytics,
};
pub use resolution_manager::ResolutionManager;
pub use types::{OptimalResolutionPrediction, ResolutionAnalysis, ResolutionEvent, ViewportStats};
