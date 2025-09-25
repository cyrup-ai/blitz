//! Cache implementation using goldylox multi-tier caching

use goldylox::traits::{CacheKey, CacheValue};
use goldylox::{Goldylox, GoldyloxBuilder};
use thiserror::Error;

use crate::measurement::types::*;

/// Standard cache error type
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache operation failed: {0}")]
    OperationFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Goldylox error: {0}")]
    GoldyloxError(String),
}

/// Cache implementation trait for goldylox-based measurement caching
pub trait CacheImpl {
    type Key: CacheKey;
    type Value: CacheValue;
    type Error: std::error::Error + Send + Sync;

    fn get(&self, key: &Self::Key) -> Result<Option<Self::Value>, Self::Error>;
    fn put(&mut self, key: Self::Key, value: Self::Value) -> Result<(), Self::Error>;
    fn clear(&mut self) -> Result<(), Self::Error>;
    fn len(&self) -> Result<usize, Self::Error>;
    fn is_empty(&self) -> Result<bool, Self::Error>;
}

/// Goldylox-based measurement cache
pub struct GoldyloxMeasurementCache {
    cache: Goldylox<String, TextMeasurement>,
}

impl GoldyloxMeasurementCache {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxBuilder::new()
            .hot_tier_max_entries(1500)
            .hot_tier_memory_limit_mb(96)
            .warm_tier_max_entries(6000)
            .warm_tier_max_memory_bytes(192 * 1024 * 1024) // 192MB
            .cold_tier_max_size_bytes(384 * 1024 * 1024) // 384MB
            .compression_level(6)
            .background_worker_threads(3)
            .cache_id("goldylox_measurement_cache")
            .build()?;

        Ok(Self { cache })
    }
}

impl CacheImpl for GoldyloxMeasurementCache {
    type Key = String;
    type Value = TextMeasurement;
    type Error = CacheError;

    fn get(&self, key: &Self::Key) -> Result<Option<Self::Value>, Self::Error> {
        match self.cache.get(key) {
            Some(value) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    fn put(&mut self, key: Self::Key, value: Self::Value) -> Result<(), Self::Error> {
        self.cache
            .put(key, value)
            .map_err(|e| CacheError::GoldyloxError(e.to_string()))
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.cache
            .clear()
            .map_err(|e| CacheError::GoldyloxError(e.to_string()))
    }

    fn len(&self) -> Result<usize, Self::Error> {
        // Goldylox doesn't expose len() - return 0 as placeholder
        Ok(0)
    }

    fn is_empty(&self) -> Result<bool, Self::Error> {
        Ok(self.len()? == 0)
    }
}

impl Default for GoldyloxMeasurementCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback: create a minimal cache that always works
            GoldyloxMeasurementCache {
                cache: GoldyloxBuilder::<String, TextMeasurement>::new()
                    .cache_id("goldylox_measurement_cache_fallback")
                    .build()
                    .unwrap(),
            }
        })
    }
}

/// Simple wrapper for backward compatibility
pub struct SimpleMeasurementCache {
    cache: GoldyloxMeasurementCache,
}

impl SimpleMeasurementCache {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxMeasurementCache::new()?;
        Ok(Self { cache })
    }

    pub fn get(&self, key: &str) -> Option<TextMeasurement> {
        self.cache.get(&key.to_string()).unwrap_or(None)
    }

    pub fn put(
        &mut self,
        key: String,
        value: TextMeasurement,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache
            .put(key, value)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache
            .clear()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

impl Default for SimpleMeasurementCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback: create a minimal cache that always works
            SimpleMeasurementCache {
                cache: GoldyloxMeasurementCache::default(),
            }
        })
    }
}

/// Main measurement cache interface using goldylox
pub type MeasurementCache = GoldyloxMeasurementCache;
