//! Core types and statistics for cursor management
//!
//! This module contains the data structures and statistics used by the cursor
//! management system for BiDi text.

use super::super::cache::{CURSOR_CACHE_HITS, CURSOR_CACHE_MISSES};

/// Cursor positioning statistics
#[derive(Debug, Clone)]
pub struct CursorStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl CursorStats {
    /// Create new cursor stats from current cache counters
    pub fn new() -> Self {
        Self {
            cache_hits: CURSOR_CACHE_HITS.load(std::sync::atomic::Ordering::Relaxed) as u64,
            cache_misses: CURSOR_CACHE_MISSES.load(std::sync::atomic::Ordering::Relaxed) as u64,
        }
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Reset all statistics to zero
    pub fn reset(&mut self) {
        self.cache_hits = 0;
        self.cache_misses = 0;
    }

    /// Update cache statistics based on hit/miss result
    pub fn update_cache_stats(&mut self, was_cache_hit: bool) {
        if was_cache_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
    }
}

impl Default for CursorStats {
    fn default() -> Self {
        Self::new()
    }
}
