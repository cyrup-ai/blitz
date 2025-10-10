//! Custom glyphs cache using goldylox multi-tier caching

use goldylox::prelude::*;
use serde::{Serialize, Deserialize};
use serde_json;
use glyphon::CustomGlyphId;

use super::types::{CustomGlyphData, GlyphKey};

// GlyphKey is no longer needed - goldylox uses String keys directly

impl CacheValue for CustomGlyphData {
    type Metadata = CacheValueMetadata;
    
    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
    
    fn is_expensive(&self) -> bool {
        false // Custom glyph data is relatively lightweight
    }
    
    fn compression_hint(&self) -> CompressionHint {
        CompressionHint::Auto
    }
    
    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

/// Custom glyphs cache using goldylox multi-tier caching
pub struct CustomGlyphsCache {
    cache: Goldylox<String, CustomGlyphData>,
}

impl CustomGlyphsCache {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache = GoldyloxBuilder::<String, CustomGlyphData>::new()
            .hot_tier_max_entries(2000)
            .hot_tier_memory_limit_mb(64)
            .warm_tier_max_entries(8000)
            .warm_tier_max_memory_bytes(256 * 1024 * 1024) // 256MB
            .cold_tier_max_size_bytes(1024 * 1024 * 1024) // 1GB
            .compression_level(2)
            .background_worker_threads(1)
            .cache_id("custom_glyphs_cache")
            .build().await?;

        Ok(Self { cache })
    }

    pub async fn get(&self, key: &str) -> Option<CustomGlyphData> {
        self.cache.get(key).await
    }

    pub async fn put(&self, key: String, value: CustomGlyphData) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache.put(key, value).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache.clear().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub fn len(&self) -> usize {
        match self.cache.detailed_analytics() {
            Ok(analytics_json) => {
                // Parse JSON to extract analyzer_tracked_keys
                if let Ok(analytics) = serde_json::from_str::<serde_json::Value>(&analytics_json) {
                    analytics["analyzer_tracked_keys"]
                        .as_u64()
                        .unwrap_or(0) as usize
                } else {
                    0
                }
            }
            Err(_) => 0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for CustomGlyphsCache {
    fn default() -> Self {
        // Since new() is async and Default can't be async, we use a blocking approach
        use tokio::runtime::Handle;
        
        // Try to use current runtime if available
        if let Ok(handle) = Handle::try_current() {
            handle.block_on(async {
                Self::new().await.unwrap_or_else(|_| {
                    panic!("Failed to create CustomGlyphsCache")
                })
            })
        } else {
            // No runtime available, create one temporarily
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(async {
                    Self::new().await.unwrap_or_else(|_| {
                        panic!("Failed to create CustomGlyphsCache")
                    })
                })
        }
    }
}