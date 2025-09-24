//! Lock-free performance monitoring and telemetry for cache system
//!
//! This module provides sophisticated zero-cost abstractions for monitoring cache
//! performance with lock-free operations and zero allocations in hot paths.
//!
//! The monitoring system has been decomposed into logical modules:
//! - ring_buffer: Lock-free ring buffer for performance history
//! - atomic_metrics: Cache-line aligned atomic metrics and snapshots
//! - sampling: Performance sample collection and trend analysis
//! - alerts: Alert types, thresholds, and notification system
//! - monitor_core: Main CachePerformanceMonitor implementation
//! - global_api: Global monitor access and convenience functions

// Module declarations
pub mod alerts;
pub mod atomic_metrics;
pub mod global_api;
pub mod monitor_core;
pub mod ring_buffer;
pub mod sampling;

// Re-export all public types for backwards compatibility
use std::sync::Arc;

pub use alerts::{AlertSeverity, AlertThresholds, AlertType, PerformanceAlert};
pub use atomic_metrics::{AtomicMetrics, CacheMetricsSnapshot};
// Re-export utilities
pub use global_api::utils;
// Re-export global API functions
pub use global_api::{
    collect_sample, get_buffer_utilization, get_config, get_history, get_metrics, get_trends,
    init_default_monitor, init_high_frequency_monitor, init_low_overhead_monitor,
    init_performance_monitor, is_monitor_initialized, is_monitoring_enabled, process_sample,
    record_error, record_memory_usage, record_operation, reset_metrics,
    shutdown_performance_monitor,
};
pub use monitor_core::CachePerformanceMonitor;
pub use ring_buffer::LockFreeRingBuffer;
pub use sampling::{MonitorConfig, PerformanceSample, PerformanceTrends};

// Common type aliases
use crate::types::ShapedText;

/// Type alias for cache operation results used in monitoring
pub type MonitoredCacheResult = super::cache::types::CacheResult<Arc<ShapedText>>;

/// Type alias for cache errors used in monitoring  
pub type MonitoredCacheError = super::cache::types::CacheOperationError;
