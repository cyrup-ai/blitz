//! BiDi cache using goldylox multi-tier caching

use std::sync::atomic::AtomicUsize;

use goldylox::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use unicode_bidi::BidiInfo;

use crate::bidi::types::*;

/// BiDi cache key for goldylox
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct BidiCacheKey {
    pub text: String,
    pub text_hash: u64,
}

impl BidiCacheKey {
    pub fn new(text: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);

        Self {
            text: text.to_string(),
            text_hash: hasher.finish(),
        }
    }
}

// BidiCacheKey is no longer needed - goldylox uses String keys directly

impl CacheValue for ProcessedBidi {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn is_expensive(&self) -> bool {
        false // BiDi processing is relatively lightweight
    }

    fn compression_hint(&self) -> CompressionHint {
        CompressionHint::Auto
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

/// BiDi cache using goldylox multi-tier caching
pub struct BidiCache {
    cache: Goldylox<String, ProcessedBidi>,
}

impl BidiCache {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CACHE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let cache_id = format!("bidi_cache_{}", CACHE_COUNTER.fetch_add(1, Ordering::Relaxed));
        
        let cache = GoldyloxBuilder::<String, ProcessedBidi>::new()
            .hot_tier_max_entries(500)
            .hot_tier_memory_limit_mb(32)
            .warm_tier_max_entries(2000)
            .warm_tier_max_memory_bytes(128 * 1024 * 1024) // 128MB
            .cold_tier_max_size_bytes(512 * 1024 * 1024) // 512MB
            .compression_level(4)
            .background_worker_threads(1)
            .cache_id(&cache_id)
            .build().await?;

        Ok(Self { cache })
    }

    pub async fn get(&self, text: &str) -> Option<ProcessedBidi> {
        self.cache.get(&text.to_string()).await
    }

    pub async fn put(
        &self,
        text: String,
        value: ProcessedBidi,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache
            .put(text, value).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache
            .clear().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub fn len(&self) -> usize {
        match self.cache.detailed_analytics() {
            Ok(analytics_json) => {
                // Parse JSON to extract analyzer_tracked_keys
                if let Ok(analytics) = serde_json::from_str::<serde_json::Value>(&analytics_json) {
                    analytics["analyzer_tracked_keys"].as_u64().unwrap_or(0) as usize
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for BidiCache {
    fn default() -> Self {
        // Since new() is async and Default can't be async, we use a blocking approach
        use tokio::runtime::Handle;
        
        // Try to use current runtime if available
        if let Ok(handle) = Handle::try_current() {
            handle.block_on(async {
                Self::new().await.unwrap_or_else(|_| {
                    panic!("Failed to create BidiCache")
                })
            })
        } else {
            // No runtime available, create one temporarily
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(async {
                    Self::new().await.unwrap_or_else(|_| {
                        panic!("Failed to create BidiCache")
                    })
                })
        }
    }
}

// Compatibility exports for existing code
pub use crate::measurement::cache::CacheManager;
pub use crate::measurement::cache::{CacheMemoryUsage, CacheStatistics};

// Global statistics for compatibility
pub static BIDI_CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
pub static BIDI_CACHE_MISSES: AtomicUsize = AtomicUsize::new(0);
pub static CURSOR_CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
pub static CURSOR_CACHE_MISSES: AtomicUsize = AtomicUsize::new(0);
