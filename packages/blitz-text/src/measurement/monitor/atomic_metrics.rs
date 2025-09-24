//! Atomic metrics for lock-free performance tracking
//!
//! This module provides cache-line aligned atomic metrics structures
//! for high-performance concurrent access without locks.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::SystemTime;

/// Lock-free atomic metrics with cache-line alignment
#[repr(align(64))]
pub struct AtomicMetrics {
    /// Total operations counter
    total_operations: AtomicU64,
    /// Hit counter  
    hits: AtomicU64,
    /// Miss counter
    misses: AtomicU64,
    /// Total access time accumulator
    total_access_time_ns: AtomicU64,
    /// Peak memory usage
    peak_memory: AtomicUsize,
    /// Current memory usage
    current_memory: AtomicUsize,
    /// Error counter
    errors: AtomicU64,
    /// Last operation timestamp (nanoseconds since epoch)
    last_operation_ns: AtomicU64,
    /// Start time for calculating rates
    start_time_ns: AtomicU64,
}

impl AtomicMetrics {
    #[inline(always)]
    pub fn new() -> Self {
        let now_ns = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        Self {
            total_operations: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            total_access_time_ns: AtomicU64::new(0),
            peak_memory: AtomicUsize::new(0),
            current_memory: AtomicUsize::new(0),
            errors: AtomicU64::new(0),
            last_operation_ns: AtomicU64::new(now_ns),
            start_time_ns: AtomicU64::new(now_ns),
        }
    }

    /// Increment total operations counter
    #[inline(always)]
    pub fn increment_operations(&self) -> u64 {
        self.total_operations.fetch_add(1, Ordering::Relaxed)
    }

    /// Increment hits counter
    #[inline(always)]
    pub fn increment_hits(&self) -> u64 {
        self.hits.fetch_add(1, Ordering::Relaxed)
    }

    /// Increment misses counter
    #[inline(always)]
    pub fn increment_misses(&self) -> u64 {
        self.misses.fetch_add(1, Ordering::Relaxed)
    }

    /// Add to total access time
    #[inline(always)]
    pub fn add_access_time(&self, time_ns: u64) {
        self.total_access_time_ns
            .fetch_add(time_ns, Ordering::Relaxed);
    }

    /// Increment errors counter
    #[inline(always)]
    pub fn increment_errors(&self) -> u64 {
        self.errors.fetch_add(1, Ordering::Relaxed)
    }

    /// Update last operation timestamp
    #[inline(always)]
    pub fn update_last_operation(&self, timestamp_ns: u64) {
        self.last_operation_ns
            .store(timestamp_ns, Ordering::Relaxed);
    }

    /// Update current memory usage
    #[inline(always)]
    pub fn update_current_memory(&self, usage: usize) {
        self.current_memory.store(usage, Ordering::Relaxed);
    }

    /// Update peak memory usage (using compare-and-swap)
    #[inline(always)]
    pub fn update_peak_memory(&self, usage: usize) {
        let mut current_peak = self.peak_memory.load(Ordering::Relaxed);
        while usage > current_peak {
            match self.peak_memory.compare_exchange_weak(
                current_peak,
                usage,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = actual,
            }
        }
    }

    /// Load total operations
    #[inline(always)]
    pub fn total_operations(&self) -> u64 {
        self.total_operations.load(Ordering::Relaxed)
    }

    /// Load hits count
    #[inline(always)]
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Load misses count
    #[inline(always)]
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Load total access time
    #[inline(always)]
    pub fn total_access_time_ns(&self) -> u64 {
        self.total_access_time_ns.load(Ordering::Relaxed)
    }

    /// Load peak memory usage
    #[inline(always)]
    pub fn peak_memory(&self) -> usize {
        self.peak_memory.load(Ordering::Relaxed)
    }

    /// Load current memory usage
    #[inline(always)]
    pub fn current_memory(&self) -> usize {
        self.current_memory.load(Ordering::Relaxed)
    }

    /// Load errors count
    #[inline(always)]
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Load last operation timestamp
    #[inline(always)]
    pub fn last_operation_ns(&self) -> u64 {
        self.last_operation_ns.load(Ordering::Relaxed)
    }

    /// Load start time
    #[inline(always)]
    pub fn start_time_ns(&self) -> u64 {
        self.start_time_ns.load(Ordering::Relaxed)
    }

    /// Reset all metrics to zero
    #[inline(always)]
    pub fn reset(&self) {
        let now_ns = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        self.total_operations.store(0, Ordering::Relaxed);
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.total_access_time_ns.store(0, Ordering::Relaxed);
        self.peak_memory.store(0, Ordering::Relaxed);
        self.current_memory.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.last_operation_ns.store(now_ns, Ordering::Relaxed);
        self.start_time_ns.store(now_ns, Ordering::Relaxed);
    }
}

impl Default for AtomicMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of cache metrics (no heap allocations)
#[derive(Debug, Clone, Copy)]
pub struct CacheMetricsSnapshot {
    pub total_operations: u64,
    pub hits: u64,
    pub misses: u64,
    pub total_access_time_ns: u64,
    pub peak_memory: usize,
    pub current_memory: usize,
    pub errors: u64,
    pub last_operation_ns: u64, // Nanoseconds since epoch
}

impl From<&AtomicMetrics> for CacheMetricsSnapshot {
    fn from(metrics: &AtomicMetrics) -> Self {
        Self {
            total_operations: metrics.total_operations(),
            hits: metrics.hits(),
            misses: metrics.misses(),
            total_access_time_ns: metrics.total_access_time_ns(),
            peak_memory: metrics.peak_memory(),
            current_memory: metrics.current_memory(),
            errors: metrics.errors(),
            last_operation_ns: metrics.last_operation_ns(),
        }
    }
}
