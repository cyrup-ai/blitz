//! Alert system for performance monitoring
//!
//! This module provides zero-allocation alert types and threshold management
//! for detecting performance issues in the cache system.

use super::sampling::PerformanceSample;

/// Zero-allocation performance alert
#[derive(Debug, Clone, Copy)]
pub struct PerformanceAlert {
    /// Alert timestamp (nanoseconds since epoch)
    pub timestamp_ns: u64,
    /// Alert type (encoded to avoid string allocations)
    pub alert_type: AlertType,
    /// Related metric value (scaled for precision)
    pub metric_value: u32,
    /// Threshold that was exceeded (scaled for precision)
    pub threshold_value: u32,
}

/// Alert types (no heap allocations)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AlertType {
    HitRateLow = 1,
    AccessTimeBig = 2,
    MemoryHigh = 3,
    OpsLow = 4,
    ErrorRateHigh = 5,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AlertSeverity {
    Info = 1,
    Warning = 2,
    Critical = 3,
}

/// Alert threshold configuration (immutable after creation)
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct AlertThresholds {
    /// Minimum acceptable hit rate (x1000 for precision)
    pub min_hit_rate_x1000: u32,
    /// Maximum acceptable access time (ns)
    pub max_access_time_ns: u32,
    /// Maximum memory usage (bytes)
    pub max_memory_bytes: u32,
    /// Minimum operations per second (x100 for precision)
    pub min_ops_per_second_x100: u32,
    /// Maximum error rate percentage (x1000 for precision)
    pub max_error_rate_x1000: u32,
    /// Padding to cache line
    _padding: [u8; 44],
}

impl PerformanceAlert {
    /// Create a new alert
    pub fn new(
        timestamp_ns: u64,
        alert_type: AlertType,
        metric_value: u32,
        threshold_value: u32,
    ) -> Self {
        Self {
            timestamp_ns,
            alert_type,
            metric_value,
            threshold_value,
        }
    }

    /// Get alert severity based on how much the threshold was exceeded
    pub fn severity(&self) -> AlertSeverity {
        let ratio = if self.threshold_value > 0 {
            self.metric_value as f32 / self.threshold_value as f32
        } else {
            1.0
        };

        match self.alert_type {
            AlertType::HitRateLow => {
                // For hit rate, lower values are worse
                let deficit_ratio = self.threshold_value as f32 / self.metric_value.max(1) as f32;
                if deficit_ratio > 2.0 {
                    AlertSeverity::Critical
                } else if deficit_ratio > 1.5 {
                    AlertSeverity::Warning
                } else {
                    AlertSeverity::Info
                }
            }
            AlertType::OpsLow => {
                // For ops rate, lower values are worse
                let deficit_ratio = self.threshold_value as f32 / self.metric_value.max(1) as f32;
                if deficit_ratio > 3.0 {
                    AlertSeverity::Critical
                } else if deficit_ratio > 2.0 {
                    AlertSeverity::Warning
                } else {
                    AlertSeverity::Info
                }
            }
            AlertType::AccessTimeBig | AlertType::MemoryHigh | AlertType::ErrorRateHigh => {
                // For these metrics, higher values are worse
                if ratio > 2.0 {
                    AlertSeverity::Critical
                } else if ratio > 1.5 {
                    AlertSeverity::Warning
                } else {
                    AlertSeverity::Info
                }
            }
        }
    }

    /// Get human-readable description of the alert
    pub fn description(&self) -> &'static str {
        match self.alert_type {
            AlertType::HitRateLow => "Cache hit rate below threshold",
            AlertType::AccessTimeBig => "Average access time above threshold",
            AlertType::MemoryHigh => "Memory usage above threshold",
            AlertType::OpsLow => "Operations per second below threshold",
            AlertType::ErrorRateHigh => "Error rate above threshold",
        }
    }
}

impl AlertThresholds {
    /// Create new alert thresholds
    pub fn new(
        min_hit_rate_percent: f32,
        max_access_time_ms: f32,
        max_memory_mb: u32,
        min_ops_per_second: f32,
        max_error_rate_percent: f32,
    ) -> Self {
        Self {
            min_hit_rate_x1000: (min_hit_rate_percent * 1000.0) as u32,
            max_access_time_ns: (max_access_time_ms * 1_000_000.0) as u32,
            max_memory_bytes: max_memory_mb * 1024 * 1024,
            min_ops_per_second_x100: (min_ops_per_second * 100.0) as u32,
            max_error_rate_x1000: (max_error_rate_percent * 1000.0) as u32,
            _padding: [0; 44],
        }
    }

    /// Check for alerts in a performance sample
    pub fn check_sample(&self, sample: &PerformanceSample) -> Vec<PerformanceAlert> {
        let mut alerts = Vec::new();

        // Check hit rate
        if sample.hit_rate_x1000 < self.min_hit_rate_x1000 {
            alerts.push(PerformanceAlert::new(
                sample.timestamp_ns,
                AlertType::HitRateLow,
                sample.hit_rate_x1000,
                self.min_hit_rate_x1000,
            ));
        }

        // Check access time
        if sample.avg_access_time_ns > self.max_access_time_ns {
            alerts.push(PerformanceAlert::new(
                sample.timestamp_ns,
                AlertType::AccessTimeBig,
                sample.avg_access_time_ns,
                self.max_access_time_ns,
            ));
        }

        // Check memory usage
        if sample.memory_usage > self.max_memory_bytes {
            alerts.push(PerformanceAlert::new(
                sample.timestamp_ns,
                AlertType::MemoryHigh,
                sample.memory_usage,
                self.max_memory_bytes,
            ));
        }

        // Check operations per second
        if sample.ops_per_second_x100 < self.min_ops_per_second_x100 {
            alerts.push(PerformanceAlert::new(
                sample.timestamp_ns,
                AlertType::OpsLow,
                sample.ops_per_second_x100,
                self.min_ops_per_second_x100,
            ));
        }

        alerts
    }

    /// Create conservative thresholds for production use
    pub fn conservative() -> Self {
        Self::new(
            80.0, // 80% minimum hit rate
            2.0,  // 2ms maximum access time
            500,  // 500MB maximum memory
            50.0, // 50 ops/sec minimum
            1.0,  // 1% maximum error rate
        )
    }

    /// Create aggressive thresholds for high-performance systems
    pub fn aggressive() -> Self {
        Self::new(
            95.0,   // 95% minimum hit rate
            0.5,    // 0.5ms maximum access time
            100,    // 100MB maximum memory
            1000.0, // 1000 ops/sec minimum
            0.1,    // 0.1% maximum error rate
        )
    }

    /// Validate threshold configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.min_hit_rate_x1000 > 100_000 {
            return Err("Minimum hit rate cannot exceed 100%".to_string());
        }

        if self.max_access_time_ns == 0 {
            return Err("Maximum access time must be greater than 0".to_string());
        }

        if self.max_memory_bytes == 0 {
            return Err("Maximum memory must be greater than 0".to_string());
        }

        if self.max_error_rate_x1000 > 100_000 {
            return Err("Maximum error rate cannot exceed 100%".to_string());
        }

        Ok(())
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            min_hit_rate_x1000: 70_000,          // 70% minimum hit rate * 1000
            max_access_time_ns: 1_000_000,       // 1ms maximum access time
            max_memory_bytes: 100 * 1024 * 1024, // 100MB maximum memory
            min_ops_per_second_x100: 10_000,     // 100 ops/sec minimum * 100
            max_error_rate_x1000: 5_000,         // 5% maximum error rate * 1000
            _padding: [0; 44],
        }
    }
}

impl AlertType {
    /// Get all possible alert types
    pub fn all() -> &'static [AlertType] {
        &[
            AlertType::HitRateLow,
            AlertType::AccessTimeBig,
            AlertType::MemoryHigh,
            AlertType::OpsLow,
            AlertType::ErrorRateHigh,
        ]
    }

    /// Get the metric name for this alert type
    pub fn metric_name(&self) -> &'static str {
        match self {
            AlertType::HitRateLow => "hit_rate",
            AlertType::AccessTimeBig => "access_time",
            AlertType::MemoryHigh => "memory_usage",
            AlertType::OpsLow => "operations_per_second",
            AlertType::ErrorRateHigh => "error_rate",
        }
    }
}
