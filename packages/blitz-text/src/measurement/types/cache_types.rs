//! Cache-specific types for text measurement with serde support
//!
//! This module defines types used in text measurement that work seamlessly
//! with the blitz-cache serde support system.

use serde::{Deserialize, Serialize};

/// Cache key for text shaping operations
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShapingCacheKey {
    pub text_hash: u64,
    pub attrs_hash: u64,
    pub max_width_hash: u64,
    pub feature_hash: u64,
}

impl ShapingCacheKey {
    pub fn new(text_hash: u64, attrs_hash: u64, max_width_hash: u64, feature_hash: u64) -> Self {
        Self {
            text_hash,
            attrs_hash,
            max_width_hash,
            feature_hash,
        }
    }
}

/// Shaped text result containing glyph runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapedText {
    pub runs: Vec<u8>, // Serialized glyph runs
    pub total_advance: f32,
    pub line_height: f32,
}

impl ShapedText {
    pub fn new(runs: Vec<u8>, total_advance: f32, line_height: f32) -> Self {
        Self {
            runs,
            total_advance,
            line_height,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub memory_usage: usize,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            memory_usage: 0,
        }
    }

    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}
