//! Global performance monitoring API
//!
//! This module provides thread-safe global access to the performance monitor
//! with lock-free operations for high-performance monitoring.

use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;

use arrayvec::ArrayVec;

use super::monitor_core::CachePerformanceMonitor;
use super::sampling::{MonitorConfig, PerformanceSample, PerformanceTrends};
use crate::measurement::cache::types::{CacheOperationError, CacheResult};
use crate::types::ShapedText;

/// Global performance monitor (lock-free access)
static PERFORMANCE_MONITOR: AtomicPtr<CachePerformanceMonitor> =
    AtomicPtr::new(std::ptr::null_mut());

/// Initialize global performance monitor (call once at startup)
pub fn init_performance_monitor(config: MonitorConfig) -> Result<(), String> {
    let monitor = Box::new(CachePerformanceMonitor::new(config)?);
    let monitor_ptr = Box::into_raw(monitor);

    let old_ptr = PERFORMANCE_MONITOR.swap(monitor_ptr, Ordering::AcqRel);
    if !old_ptr.is_null() {
        // Clean up old monitor if one existed
        let _ = unsafe { Box::from_raw(old_ptr) };
    }

    Ok(())
}

/// Initialize with default configuration
pub fn init_default_monitor() -> Result<(), String> {
    init_performance_monitor(MonitorConfig::default())
}

/// Initialize with high-frequency monitoring
pub fn init_high_frequency_monitor() -> Result<(), String> {
    init_performance_monitor(MonitorConfig::high_frequency())
}

/// Initialize with low-overhead monitoring
pub fn init_low_overhead_monitor() -> Result<(), String> {
    init_performance_monitor(MonitorConfig::low_overhead())
}

/// Shutdown the global performance monitor
pub fn shutdown_performance_monitor() {
    let old_ptr = PERFORMANCE_MONITOR.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !old_ptr.is_null() {
        let _ = unsafe { Box::from_raw(old_ptr) };
    }
}

/// Get global performance monitor (lock-free)
#[inline(always)]
fn get_performance_monitor() -> Option<&'static CachePerformanceMonitor> {
    let ptr = PERFORMANCE_MONITOR.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*ptr })
    }
}

/// Check if global monitor is initialized
#[inline(always)]
pub fn is_monitor_initialized() -> bool {
    !PERFORMANCE_MONITOR.load(Ordering::Acquire).is_null()
}

/// Record operation with global monitor (lock-free)
#[inline(always)]
pub fn record_operation(result: &CacheResult<Arc<ShapedText>>) {
    if let Some(monitor) = get_performance_monitor() {
        monitor.record_operation(result);
    }
}

/// Record memory usage with global monitor (lock-free)
#[inline(always)]
pub fn record_memory_usage(usage: usize) {
    if let Some(monitor) = get_performance_monitor() {
        monitor.record_memory_usage(usage);
    }
}

/// Record error with global monitor (lock-free)
#[inline(always)]
pub fn record_error(error: &CacheOperationError) {
    if let Some(monitor) = get_performance_monitor() {
        monitor.record_error(error);
    }
}

/// Collect sample with global monitor (lock-free)
#[inline(always)]
pub fn collect_sample() -> Option<PerformanceSample> {
    get_performance_monitor().map(|m| m.collect_sample())
}

/// Process a sample with alerting
#[inline(always)]
pub fn process_sample() -> Option<PerformanceSample> {
    get_performance_monitor().map(|m| m.process_sample())
}

/// Get performance trends with preallocated buffer
#[inline(always)]
pub fn get_trends(
    history_buffer: &mut ArrayVec<PerformanceSample, 1024>,
) -> Option<PerformanceTrends> {
    get_performance_monitor().map(|m| m.calculate_trends(history_buffer))
}

/// Get performance history with preallocated buffer
#[inline(always)]
pub fn get_history(history_buffer: &mut ArrayVec<PerformanceSample, 1024>) -> Option<usize> {
    get_performance_monitor().map(|m| m.get_history(history_buffer))
}

/// Reset all metrics in global monitor
#[inline(always)]
pub fn reset_metrics() {
    if let Some(monitor) = get_performance_monitor() {
        monitor.reset_metrics();
    }
}

/// Get current metrics snapshot
#[inline(always)]
pub fn get_metrics() -> Option<super::atomic_metrics::CacheMetricsSnapshot> {
    get_performance_monitor().map(|m| m.get_metrics())
}

/// Get monitor configuration
#[inline(always)]
pub fn get_config() -> Option<MonitorConfig> {
    get_performance_monitor().map(|m| m.config())
}

/// Get buffer utilization (for monitoring the monitor)
#[inline(always)]
pub fn get_buffer_utilization() -> Option<f32> {
    get_performance_monitor().map(|m| m.buffer_utilization())
}

/// Check if monitoring is enabled
#[inline(always)]
pub fn is_monitoring_enabled() -> bool {
    get_performance_monitor().map_or(false, |m| m.is_enabled())
}

/// Convenience macro for recording operations
#[macro_export]
macro_rules! record_cache_op {
    ($result:expr) => {
        $crate::measurement::monitor::record_operation($result)
    };
}

/// Convenience macro for recording memory usage
#[macro_export]
macro_rules! record_memory {
    ($usage:expr) => {
        $crate::measurement::monitor::record_memory_usage($usage)
    };
}

/// Convenience macro for recording errors
#[macro_export]
macro_rules! record_cache_error {
    ($error:expr) => {
        $crate::measurement::monitor::record_error($error)
    };
}

/// Performance monitoring utilities
pub mod utils {
    use std::time::Instant;

    use super::*;

    /// Simple performance timer for measuring operation durations
    pub struct PerformanceTimer {
        start: Instant,
        name: &'static str,
    }

    impl PerformanceTimer {
        /// Start a new timer
        pub fn start(name: &'static str) -> Self {
            Self {
                start: Instant::now(),
                name,
            }
        }

        /// Get elapsed time in nanoseconds
        pub fn elapsed_ns(&self) -> u64 {
            self.start.elapsed().as_nanos() as u64
        }

        /// Get elapsed time in microseconds
        pub fn elapsed_us(&self) -> u64 {
            self.start.elapsed().as_micros() as u64
        }

        /// Get elapsed time in milliseconds
        pub fn elapsed_ms(&self) -> u64 {
            self.start.elapsed().as_millis() as u64
        }

        /// Get timer name
        pub fn name(&self) -> &'static str {
            self.name
        }
    }

    impl Drop for PerformanceTimer {
        fn drop(&mut self) {
            if is_monitoring_enabled() {
                let elapsed = self.elapsed_ns();
                // Could log or record timing information here
                if elapsed > 1_000_000 {
                    // Log operations taking more than 1ms
                    eprintln!(
                        "Performance warning: {} took {}μs",
                        self.name,
                        elapsed / 1000
                    );
                }
            }
        }
    }

    /// Macro for easy timing of operations
    #[macro_export]
    macro_rules! time_operation {
        ($name:expr, $operation:expr) => {{
            let _timer = $crate::measurement::monitor::utils::PerformanceTimer::start($name);
            $operation
        }};
    }
}
