//! Measurement cache management for text metrics
//!
//! This module provides caching functionality for text measurement operations
//! to optimize font metrics and layout calculations.

use goldylox::{Goldylox, GoldyloxBuilder};

use crate::measurement::types::{MeasurementCacheKey, TextMeasurement};

/// Cache manager for text measurement results
pub struct CacheManager {
    cache: Goldylox<String, TextMeasurement>,
}

impl CacheManager {
    /// Convert MeasurementCacheKey to String for goldylox
    fn key_to_string(key: &MeasurementCacheKey) -> String {
        serde_json::to_string(key).unwrap_or_else(|_| format!("{:?}", key))
    }
}

impl CacheManager {
    /// Create a new measurement cache manager using global cache
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use the global text measurement cache instead of creating a new one
        let cache = crate::cache::get_text_measurement_cache();
        
        println!("✅ CacheManager using global Goldylox cache (singleton)");

        Ok(Self { 
            cache: (*cache).clone() // Clone the Arc to get the underlying Goldylox instance
        })
    }

    /// Get cached measurement result
    pub async fn get(&self, key: &MeasurementCacheKey) -> Option<TextMeasurement> {
        let string_key = Self::key_to_string(key);
        self.cache.get(&string_key).await
    }

    /// Store measurement result in cache
    pub async fn put(
        &self,
        key: MeasurementCacheKey,
        value: TextMeasurement,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let string_key = Self::key_to_string(&key);
        self.cache
            .put(string_key, value).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.cache.stats()?)
    }

    /// Clear all cached measurements
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache
            .clear().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Get cache statistics (static method)
    pub fn get_cache_stats() -> CacheStatistics {
        // Return default stats since we need static access
        CacheStatistics::default()
    }

    /// Get cache memory usage (static method)
    pub fn get_cache_memory_usage() -> CacheMemoryUsage {
        // Return default memory usage since we need static access
        CacheMemoryUsage::default()
    }

    /// Store cursor cached result (static method)
    pub fn store_cursor_cached<K, V>(_key: K, _value: V) {
        // Placeholder implementation for static method
    }

    /// Store bidi cached result (static method)
    pub fn store_bidi_cached<K, V>(_key: K, _value: V) {
        // Placeholder implementation for static method
    }

    /// Get cursor cached result (static method)
    pub fn get_cursor_cached<K, V>(_key: &K) -> Option<V> {
        // Placeholder implementation for static method
        None
    }

    /// Get bidi cached result (static method)
    pub fn get_bidi_cached<K, V>(_key: &K) -> Option<V> {
        // Placeholder implementation for static method
        None
    }

    /// Clear all caches (static method)
    pub fn clear_all_caches() {
        // Placeholder implementation for static method
    }

    /// Cache measurement (instance method)
    pub fn cache_measurement<K, V>(&self, _key: K, _value: V) {
        // Placeholder implementation
    }

    /// Get measurement (instance method)
    pub fn get_measurement<K, V>(&self, _key: &K) -> Option<V> {
        // Placeholder implementation
        None
    }

    /// Get stats (instance method)
    pub fn get_stats(&self) -> CacheStatistics {
        CacheStatistics::default()
    }

    /// Cache baseline (instance method)
    pub fn cache_baseline<K, V>(&self, _key: K, _value: V) {
        // Placeholder implementation
    }

    /// Get baseline (instance method)
    pub fn get_baseline<K, V>(&self, _key: &K) -> Option<V> {
        // Placeholder implementation
        None
    }

    /// Cache font metrics (instance method)
    pub fn cache_font_metrics<K, V>(&self, _key: K, _value: V) {
        // Placeholder implementation
    }

    /// Get font metrics (instance method)
    pub fn get_font_metrics<K, V>(&self, _key: &K) -> Option<V> {
        // Placeholder implementation
        None
    }

    /// Create cache manager with memory limit
    pub async fn with_memory_limit(
        memory_mb: u64,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache = GoldyloxBuilder::<String, TextMeasurement>::new()
            .hot_tier_memory_limit_mb(memory_mb as u32)
            .build().await?;
        Ok(Self { cache })
    }

    /// Create cache key for measurement requests
    pub fn create_cache_key<T>(&self, request: &T) -> String
    where
        T: std::fmt::Debug,
    {
        format!("{:?}", request)
    }

    /// Check if result should be cached
    pub fn should_cache<T>(&self, _item: &T) -> bool {
        // Cache eligibility heuristics:
        // 1. Check item size (Goldylox handles size estimation via CacheValue trait)
        // 2. Very small items might not benefit from caching overhead
        // 3. Goldylox's built-in priority and eviction handles most optimization
        // 4. Main concern: avoid caching truly ephemeral data
        
        let estimated_size = std::mem::size_of::<T>();
        
        // Don't cache very small items (< 16 bytes) - overhead not worth it
        if estimated_size < 16 {
            return false;
        }
        
        // Don't cache extremely large items (> 1MB) - they'll dominate cache
        if estimated_size > 1024 * 1024 {
            return false;
        }
        
        // Everything else is cacheable - Goldylox priority system handles optimization
        true
    }

    /// Optimize cache performance
    pub fn optimize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Goldylox handles optimization internally
        Ok(())
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        // Since build() is async and Default can't be async, we use a blocking approach
        use tokio::runtime::Handle;
        
        // Try to use current runtime if available
        if let Ok(handle) = Handle::try_current() {
            handle.block_on(async {
                let cache = GoldyloxBuilder::<String, TextMeasurement>::new()
                    .hot_tier_max_entries(2000)
                    .hot_tier_memory_limit_mb(32)
                    .warm_tier_max_entries(8000)
                    .warm_tier_max_memory_bytes(64 * 1024 * 1024) // 64MB
                    .cold_tier_max_size_bytes(128 * 1024 * 1024) // 128MB
                    .compression_level(4)
                    .background_worker_threads(2)
                    .cache_id("measurement_cache_default")
                    .build().await
                    .unwrap_or_else(|_| {
                        // Last resort: minimal cache configuration
                        tokio::runtime::Runtime::new()
                            .expect("Failed to create tokio runtime")
                            .block_on(async {
                                GoldyloxBuilder::<String, TextMeasurement>::new()
                                    .cache_id("measurement_cache_minimal")
                                    .build().await
                                    .unwrap()
                            })
                    });
                Self { cache }
            })
        } else {
            // No runtime available, create one temporarily
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(async {
                    let cache = GoldyloxBuilder::<String, TextMeasurement>::new()
                        .hot_tier_max_entries(2000)
                        .hot_tier_memory_limit_mb(32)
                        .warm_tier_max_entries(8000)
                        .warm_tier_max_memory_bytes(64 * 1024 * 1024) // 64MB
                        .cold_tier_max_size_bytes(128 * 1024 * 1024) // 128MB
                        .compression_level(4)
                        .background_worker_threads(2)
                        .cache_id("measurement_cache_default")
                        .build().await
                        .expect("Failed to build cache");
                    Self { cache }
                })
        }
    }
}

/// Measurement cache memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct CacheMemoryUsage {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub cached_items: usize,
}

/// Measurement cache performance statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub memory_usage: CacheMemoryUsage,
}



/// Cache types module for measurement operations
pub mod types {
    pub use goldylox::prelude::CacheOperationError;

    pub use super::{CacheManager, CacheMemoryUsage, CacheStatistics};

    /// Cache result type alias
    pub type CacheResult<T> = Result<T, CacheOperationError>;

    /// Hit status for cache operations
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HitStatus {
        Hit,
        Miss,
        Error,
        Partial,
    }
}

/// Unified cache manager aggregating all blitz-text cache types
pub struct UnifiedCacheManager {
    pub measurement_cache: CacheManager,
    pub font_metrics_cache: crate::measurement::enhanced::font_metrics::FontMetricsCache,
    pub bidi_cache: crate::bidi::cache::BidiCache,
    pub features_cache: crate::features::cache::FeaturesCache,
}

impl UnifiedCacheManager {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            measurement_cache: CacheManager::new().map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?,
            font_metrics_cache: crate::measurement::enhanced::font_metrics::FontMetricsCache::new().await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?,
            bidi_cache: crate::bidi::cache::BidiCache::new().await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?,
            features_cache: crate::features::cache::FeaturesCache::new().await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?,
        })
    }

    // Delegation methods for measurement cache
    pub fn get_measurement<K, V>(&self, key: &K) -> Option<V> {
        self.measurement_cache.get_measurement(key)
    }

    pub fn cache_measurement<K, V>(&self, key: K, value: V) {
        self.measurement_cache.cache_measurement(key, value)
    }

    pub fn get_font_metrics<K, V>(&self, key: &K) -> Option<V> {
        self.measurement_cache.get_font_metrics(key)
    }

    pub fn cache_font_metrics<K, V>(&self, key: K, value: V) {
        self.measurement_cache.cache_font_metrics(key, value)
    }

    pub fn get_baseline<K, V>(&self, key: &K) -> Option<V> {
        self.measurement_cache.get_baseline(key)
    }

    pub fn cache_baseline<K, V>(&self, key: K, value: V) {
        self.measurement_cache.cache_baseline(key, value)
    }
}
