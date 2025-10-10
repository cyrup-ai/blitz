//! Cache implementation using goldylox multi-tier caching

use goldylox::traits::{CacheKey, CacheValue};
use goldylox::{Goldylox, GoldyloxBuilder};
use thiserror::Error;
use core::future::Future;

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

use std::pin::Pin;

/// Cache implementation trait for goldylox-based measurement caching (async via pinned boxed futures)
pub trait CacheImpl {
    type Key: CacheKey;
    type Value: CacheValue;
    type Error: std::error::Error + Send + Sync;

    fn get<'a>(&'a self, key: &'a Self::Key) -> Pin<Box<dyn Future<Output = Result<Option<Self::Value>, Self::Error>> + 'a>>;
    fn put<'a>(&'a mut self, key: Self::Key, value: Self::Value) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'a>>;
    fn clear<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'a>>;

    fn len(&self) -> Result<usize, Self::Error>;
    fn is_empty(&self) -> Result<bool, Self::Error>;
}

/// Goldylox-based measurement cache
pub struct GoldyloxMeasurementCache {
    cache: Goldylox<String, TextMeasurement>,
}

impl GoldyloxMeasurementCache {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxBuilder::new()
            .hot_tier_max_entries(1500)
            .hot_tier_memory_limit_mb(96)
            .warm_tier_max_entries(6000)
            .warm_tier_max_memory_bytes(192 * 1024 * 1024) // 192MB
            .cold_tier_max_size_bytes(384 * 1024 * 1024) // 384MB
            .compression_level(6)
            .background_worker_threads(3)
            .cache_id("goldylox_measurement_cache")
            .build()
            .await?;

        Ok(Self { cache })
    }
}

impl CacheImpl for GoldyloxMeasurementCache {
    type Key = String;
    type Value = TextMeasurement;
    type Error = CacheError;

    fn get<'a>(&'a self, key: &'a Self::Key) -> Pin<Box<dyn Future<Output = Result<Option<Self::Value>, Self::Error>> + 'a>> {
        Box::pin(async move {
            Ok(self.cache.get(key).await)
        })
    }

    fn put<'a>(&'a mut self, key: Self::Key, value: Self::Value) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'a>> {
        Box::pin(async move {
            self.cache
                .put(key, value)
                .await
                .map_err(|e| CacheError::GoldyloxError(e.to_string()))
        })
    }

    fn clear<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'a>> {
        Box::pin(async move {
            self.cache
                .clear()
                .await
                .map_err(|e| CacheError::GoldyloxError(e.to_string()))
        })
    }

    fn len(&self) -> Result<usize, Self::Error> {
        // Goldylox doesn't expose len() - return 0 as placeholder
        Ok(0)
    }

    fn is_empty(&self) -> Result<bool, Self::Error> {
        Ok(self.len()? == 0)
    }
}

/// Simple wrapper for backward compatibility
pub struct SimpleMeasurementCache {
    cache: GoldyloxMeasurementCache,
}

impl SimpleMeasurementCache {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxMeasurementCache::new().await?;
        Ok(Self { cache })
    }

    pub async fn get(&self, key: &str) -> Option<TextMeasurement> {
        self.cache.get(&key.to_string()).await.unwrap_or(None)
    }

    pub async fn put(
        &mut self,
        key: String,
        value: TextMeasurement,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache
            .put(key, value)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub async fn clear(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache
            .clear()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

/// Main measurement cache interface using goldylox
pub type MeasurementCache = GoldyloxMeasurementCache;
