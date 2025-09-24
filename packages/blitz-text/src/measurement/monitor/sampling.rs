//! Performance sampling and trend analysis
//!
//! This module provides structures and functions for collecting performance
//! samples and calculating trends with zero allocations.

use std::time::SystemTime;

/// Cache-aligned performance sample (64 bytes = 1 cache line)
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct PerformanceSample {
    /// Timestamp as nanoseconds since epoch
    pub timestamp_ns: u64,
    /// Cache hit rate (packed as u32 for atomic updates)
    pub hit_rate_x1000: u32, // hit_rate * 1000 for precision
    /// Average access time in nanoseconds
    pub avg_access_time_ns: u32,
    /// Memory usage in bytes
    pub memory_usage: u32,
    /// Operations per second (packed)
    pub ops_per_second_x100: u32, // ops_per_second * 100
    /// Tier utilizations packed as u16 (percentage * 100)
    pub hot_utilization_x100: u16,
    pub warm_utilization_x100: u16,
    pub cold_utilization_x100: u16,
    /// Padding to complete cache line
    _padding: [u8; 6],
}

/// Performance trend indicators (integer arithmetic for speed)
#[derive(Debug, Clone, Copy)]
pub struct PerformanceTrends {
    /// Hit rate change (scaled by 1000)
    pub hit_rate_trend: i32,
    /// Access time change (nanoseconds)
    pub access_time_trend: i64,
    /// Memory usage change (bytes)
    pub memory_trend: i64,
    /// Operations per second change (scaled by 100)
    pub ops_trend: i32,
}

/// Monitor configuration (immutable after creation)
#[derive(Debug, Clone, Copy)]
pub struct MonitorConfig {
    /// Sample collection interval (nanoseconds)
    pub sample_interval_ns: u64,
    /// Maximum history samples (must be power of 2)
    pub max_history_samples: usize,
    /// Enable detailed tracing
    pub enable_tracing: bool,
    /// Enable alerting
    pub enable_alerts: bool,
}

impl PerformanceSample {
    /// Create a new performance sample from metrics
    pub fn new(
        total_ops: u64,
        hits: u64,
        misses: u64,
        total_time_ns: u64,
        current_memory: usize,
        start_time_ns: u64,
        last_op_ns: u64,
    ) -> Self {
        let now_ns = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        // Calculate hit rate (packed for precision)
        let hit_rate_x1000 = if total_ops > 0 {
            ((hits * 100000) / total_ops) as u32 // hit_rate * 1000
        } else {
            0
        };

        // Calculate average access time
        let avg_access_time_ns = if total_ops > 0 {
            (total_time_ns / total_ops) as u32
        } else {
            0
        };

        // Calculate operations per second
        let elapsed_ns = if last_op_ns > start_time_ns {
            last_op_ns - start_time_ns
        } else {
            1 // Avoid division by zero
        };

        let ops_per_second_x100 = if elapsed_ns > 0 {
            ((total_ops * 100 * 1_000_000_000) / elapsed_ns) as u32
        } else {
            0
        };

        // Calculate tier utilizations based on current cache statistics
        let hot_utilization_x100 = if total_ops > 0 {
            // Hot tier utilization: percentage of hits that came from hot cache
            // Estimate hot tier contribution as 60% of total hits for high-performance access
            let estimated_hot_hits = (hits * 60) / 100;
            ((estimated_hot_hits * 10000) / total_ops) as u16 // utilization * 100
        } else {
            0u16
        };

        let warm_utilization_x100 = if total_ops > 0 {
            // Warm tier utilization: remaining hits distributed between warm and cold
            // Estimate warm tier as 30% of total hits for medium-speed access
            let estimated_warm_hits = (hits * 30) / 100;
            ((estimated_warm_hits * 10000) / total_ops) as u16 // utilization * 100
        } else {
            0u16
        };

        let cold_utilization_x100 = if total_ops > 0 {
            // Cold tier utilization: remaining 10% of hits plus all misses
            let estimated_cold_hits = (hits * 10) / 100;
            let cold_operations = estimated_cold_hits + misses;
            ((cold_operations * 10000) / total_ops) as u16 // utilization * 100
        } else {
            0u16
        };

        Self {
            timestamp_ns: now_ns,
            hit_rate_x1000,
            avg_access_time_ns,
            memory_usage: current_memory as u32,
            ops_per_second_x100,
            hot_utilization_x100,
            warm_utilization_x100,
            cold_utilization_x100,
            _padding: [0; 6],
        }
    }

    /// Convert packed hit rate back to percentage
    #[inline(always)]
    pub fn hit_rate(&self) -> f32 {
        self.hit_rate_x1000 as f32 / 1000.0
    }

    /// Convert packed ops per second back to float
    #[inline(always)]
    pub fn ops_per_second(&self) -> f32 {
        self.ops_per_second_x100 as f32 / 100.0
    }

    /// Convert packed utilizations back to percentages
    #[inline(always)]
    pub fn hot_utilization(&self) -> f32 {
        self.hot_utilization_x100 as f32 / 100.0
    }

    #[inline(always)]
    pub fn warm_utilization(&self) -> f32 {
        self.warm_utilization_x100 as f32 / 100.0
    }

    #[inline(always)]
    pub fn cold_utilization(&self) -> f32 {
        self.cold_utilization_x100 as f32 / 100.0
    }
}

impl PerformanceTrends {
    /// Calculate trends from two performance samples
    pub fn from_samples(recent: &PerformanceSample, previous: &PerformanceSample) -> Self {
        Self {
            hit_rate_trend: recent.hit_rate_x1000 as i32 - previous.hit_rate_x1000 as i32,
            access_time_trend: recent.avg_access_time_ns as i64
                - previous.avg_access_time_ns as i64,
            memory_trend: recent.memory_usage as i64 - previous.memory_usage as i64,
            ops_trend: recent.ops_per_second_x100 as i32 - previous.ops_per_second_x100 as i32,
        }
    }

    /// Check if performance is improving
    #[inline(always)]
    pub fn is_improving(&self) -> bool {
        self.hit_rate_trend > 0 && self.access_time_trend < 0 && self.ops_trend > 0
    }

    /// Check if performance is degrading
    #[inline(always)]
    pub fn is_degrading(&self) -> bool {
        self.hit_rate_trend < -100 || self.access_time_trend > 10_000 || self.ops_trend < -100
    }
}

impl Default for PerformanceTrends {
    fn default() -> Self {
        Self {
            hit_rate_trend: 0,
            access_time_trend: 0,
            memory_trend: 0,
            ops_trend: 0,
        }
    }
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            sample_interval_ns: 10_000_000_000, // 10 seconds in nanoseconds
            max_history_samples: 1024,          // Power of 2 for ring buffer
            enable_tracing: false,
            enable_alerts: true,
        }
    }
}

impl MonitorConfig {
    /// Create a configuration for high-frequency monitoring
    pub fn high_frequency() -> Self {
        Self {
            sample_interval_ns: 1_000_000_000, // 1 second
            max_history_samples: 2048,
            enable_tracing: true,
            enable_alerts: true,
        }
    }

    /// Create a configuration for low-overhead monitoring
    pub fn low_overhead() -> Self {
        Self {
            sample_interval_ns: 60_000_000_000, // 60 seconds
            max_history_samples: 256,
            enable_tracing: false,
            enable_alerts: false,
        }
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), String> {
        if !self.max_history_samples.is_power_of_two() {
            return Err("max_history_samples must be a power of 2".to_string());
        }

        if self.sample_interval_ns == 0 {
            return Err("sample_interval_ns must be greater than 0".to_string());
        }

        if self.max_history_samples < 16 {
            return Err("max_history_samples must be at least 16".to_string());
        }

        Ok(())
    }
}
