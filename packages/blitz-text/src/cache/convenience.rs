//! Convenience functions and preload cache
//!
//! This module provides global convenience functions for cache access
//! and a preload cache system for common text patterns.

use std::sync::Arc;

use super::manager::CacheManager;
use super::operations::CacheOperations;
use super::types::CacheStats;
use crate::error::ShapingError;
use crate::types::{ShapedText, ShapingCacheKey};

/// Convenience functions for global cache access
#[inline]
pub fn get_cached_text(key: &ShapingCacheKey) -> Option<Arc<ShapedText>> {
    CacheManager::new().get(key)
}

pub fn cache_shaped_text(key: ShapingCacheKey, value: Arc<ShapedText>) -> Result<(), ShapingError> {
    CacheManager::new().put(key, value)
}

pub fn clear_cache() {
    CacheManager::new().clear();
}

#[inline]
pub fn cache_stats() -> CacheStats {
    CacheManager::new().stats()
}

/// Preload cache with common patterns for optimal performance
pub struct PreloadCache {
    common_patterns: &'static [(&'static str, f32)],
}

impl PreloadCache {
    /// Create preload cache with common text patterns
    pub const fn new() -> Self {
        Self {
            common_patterns: &[
                ("Hello World", 16.0),
                ("The quick brown fox jumps over the lazy dog", 16.0),
                ("Lorem ipsum dolor sit amet", 16.0),
                ("0123456789", 16.0),
                ("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 16.0),
                ("abcdefghijklmnopqrstuvwxyz", 16.0),
            ],
        }
    }

    /// Get common patterns for preloading
    #[inline]
    pub fn patterns(&self) -> &'static [(&'static str, f32)] {
        self.common_patterns
    }
}

impl Default for PreloadCache {
    fn default() -> Self {
        Self::new()
    }
}
