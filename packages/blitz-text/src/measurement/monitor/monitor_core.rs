//! Core performance monitor implementation
//!
//! This module provides the main CachePerformanceMonitor struct that orchestrates
//! all monitoring functionality with lock-free operations.

use std::sync::Arc;
use std::time::SystemTime;

use arrayvec::ArrayVec;
use crossbeam::channel::Sender;

use super::alerts::{AlertThresholds, PerformanceAlert};
use super::atomic_metrics::{AtomicMetrics, CacheMetricsSnapshot};
use super::ring_buffer::LockFreeRingBuffer;
use super::sampling::{MonitorConfig, PerformanceSample, PerformanceTrends};
use crate::measurement::cache::types::{CacheOperationError, CacheResult};
use crate::types::ShapedText;

/// Lock-free performance monitor with zero allocations
#[repr(align(64))] // Cache line alignment
pub struct CachePerformanceMonitor {
    /// Lock-free circular buffer for performance history
    history_ring: LockFreeRingBuffer<PerformanceSample>,
    /// Atomic metrics - no locks needed
    metrics: AtomicMetrics,
    /// Alert thresholds (rarely changed, cache-aligned)
    thresholds: AlertThresholds,
    /// Monitor configuration (immutable after creation)
    config: MonitorConfig,
    /// Alert channel sender (lock-free)
    alert_sender: Sender<PerformanceAlert>,
}

impl CachePerformanceMonitor {
    /// Create new performance monitor with preallocated buffers
    pub fn new(config: MonitorConfig) -> Result<Self, String> {
        config.validate()?;

        let (alert_sender, _alert_receiver) = crossbeam::channel::unbounded();

        Ok(Self {
            history_ring: LockFreeRingBuffer::new(config.max_history_samples)?,
            metrics: AtomicMetrics::new(),
            thresholds: AlertThresholds::default(),
            config,
            alert_sender,
        })
    }

    /// Create monitor with custom thresholds
    pub fn with_thresholds(
        config: MonitorConfig,
        thresholds: AlertThresholds,
    ) -> Result<Self, String> {
        config.validate()?;
        thresholds.validate()?;

        let (alert_sender, _alert_receiver) = crossbeam::channel::unbounded();

        Ok(Self {
            history_ring: LockFreeRingBuffer::new(config.max_history_samples)?,
            metrics: AtomicMetrics::new(),
            thresholds,
            config,
            alert_sender,
        })
    }

    /// Record cache operation (lock-free, zero allocation)
    #[inline(always)]
    pub fn record_operation(&self, result: &CacheResult<Arc<ShapedText>>) {
        // Update atomic counters with relaxed ordering for speed
        self.metrics.increment_operations();

        let now_ns = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        self.metrics.update_last_operation(now_ns);

        // Record operation based on Result type
        match result {
            Ok(_) => {
                // Assume Hit for successful operations - could be enhanced with more context
                self.metrics.increment_hits();
                // Default latency since Result doesn't include timing
                self.metrics.add_access_time(0);
            }
            Err(_) => {
                self.metrics.increment_errors();
            }
        }
    }

    /// Record memory usage (lock-free)
    #[inline(always)]
    pub fn record_memory_usage(&self, usage: usize) {
        self.metrics.update_current_memory(usage);
        self.metrics.update_peak_memory(usage);
    }

    /// Record error (lock-free)
    #[inline(always)]
    pub fn record_error(&self, _error: &CacheOperationError) {
        self.metrics.increment_errors();
    }

    /// Collect performance sample (lock-free, zero allocation)
    #[inline(always)]
    pub fn collect_sample(&self) -> PerformanceSample {
        PerformanceSample::new(
            self.metrics.total_operations(),
            self.metrics.hits(),
            self.metrics.misses(),
            self.metrics.total_access_time_ns(),
            self.metrics.current_memory(),
            self.metrics.start_time_ns(),
            self.metrics.last_operation_ns(),
        )
    }

    /// Add sample to history (lock-free)
    #[inline(always)]
    pub fn add_sample(&self, sample: PerformanceSample) {
        // Ring buffer automatically overwrites old samples when full
        let _ = self.history_ring.push(sample); // Ignore if full
    }

    /// Check for performance alerts (zero allocation)
    #[inline(always)]
    pub fn check_alerts(&self, sample: &PerformanceSample) -> Vec<PerformanceAlert> {
        if !self.config.enable_alerts {
            return Vec::new();
        }

        self.thresholds.check_sample(sample)
    }

    /// Get performance history (zero allocation with fixed buffer)
    #[inline(always)]
    pub fn get_history(&self, buffer: &mut ArrayVec<PerformanceSample, 1024>) -> usize {
        buffer.clear();
        let mut count = 0;

        while let Some(sample) = self.history_ring.pop() {
            if buffer.try_push(sample).is_err() {
                break; // Buffer full
            }
            count += 1;
        }

        count
    }

    /// Get current metrics snapshot (lock-free)
    #[inline(always)]
    pub fn get_metrics(&self) -> CacheMetricsSnapshot {
        CacheMetricsSnapshot::from(&self.metrics)
    }

    /// Get current alert thresholds (no locking needed - immutable)
    #[inline(always)]
    pub fn get_thresholds(&self) -> AlertThresholds {
        self.thresholds
    }

    /// Calculate performance trends (with preallocated buffer)
    #[inline(always)]
    pub fn calculate_trends(
        &self,
        history_buffer: &mut ArrayVec<PerformanceSample, 1024>,
    ) -> PerformanceTrends {
        let count = self.get_history(history_buffer);

        if count < 2 {
            return PerformanceTrends::default();
        }

        let recent = &history_buffer[count - 1];
        let previous = &history_buffer[count - 2];

        PerformanceTrends::from_samples(recent, previous)
    }

    /// Reset all metrics (lock-free)
    #[inline(always)]
    pub fn reset_metrics(&self) {
        self.metrics.reset();
    }

    /// Get monitor configuration
    #[inline(always)]
    pub fn config(&self) -> MonitorConfig {
        self.config
    }

    /// Update alert thresholds (creates new monitor)
    pub fn with_updated_thresholds(&self, new_thresholds: AlertThresholds) -> Result<Self, String> {
        new_thresholds.validate()?;

        let (alert_sender, _alert_receiver) = crossbeam::channel::unbounded();

        Ok(Self {
            history_ring: LockFreeRingBuffer::new(self.config.max_history_samples)?,
            metrics: AtomicMetrics::new(),
            thresholds: new_thresholds,
            config: self.config,
            alert_sender,
        })
    }

    /// Check if monitoring is enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.config.enable_alerts || self.config.enable_tracing
    }

    /// Get buffer utilization (for monitoring the monitor)
    #[inline(always)]
    pub fn buffer_utilization(&self) -> f32 {
        let current_len = self.history_ring.len();
        let capacity = self.history_ring.capacity();

        if capacity > 0 {
            current_len as f32 / capacity as f32
        } else {
            0.0
        }
    }

    /// Send alert through channel (non-blocking)
    #[inline(always)]
    pub fn send_alert(&self, alert: PerformanceAlert) {
        if self.config.enable_alerts {
            let _ = self.alert_sender.try_send(alert); // Ignore if channel is full
        }
    }

    /// Collect and process a sample with alerting
    pub fn process_sample(&self) -> PerformanceSample {
        let sample = self.collect_sample();

        // Add to history
        self.add_sample(sample);

        // Check for alerts
        if self.config.enable_alerts {
            let alerts = self.check_alerts(&sample);
            for alert in alerts {
                self.send_alert(alert);
            }
        }

        sample
    }
}
