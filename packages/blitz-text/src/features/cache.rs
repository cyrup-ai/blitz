//! Features cache using goldylox multi-tier caching

use goldylox::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;

/// Features cache key for goldylox
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct FeaturesCacheKey {
    pub feature_name: String,
    pub context_hash: u64,
}

impl FeaturesCacheKey {
    pub fn new(feature_name: &str, context: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        context.hash(&mut hasher);

        Self {
            feature_name: feature_name.to_string(),
            context_hash: hasher.finish(),
        }
    }
}

// FeaturesCacheKey is no longer needed - goldylox uses String keys directly

/// Features cache value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesValue {
    pub features: Vec<String>,
    #[serde(skip, default = "std::time::Instant::now")]
    pub cached_at: std::time::Instant,
}

impl Default for FeaturesValue {
    fn default() -> Self {
        Self {
            features: Vec::new(),
            cached_at: std::time::Instant::now(),
        }
    }
}

impl CacheValue for FeaturesValue {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.features.iter().map(|s| s.len()).sum::<usize>()
    }

    fn is_expensive(&self) -> bool {
        self.features.len() > 10 // Many features are expensive
    }

    fn compression_hint(&self) -> CompressionHint {
        CompressionHint::Auto
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

/// Features cache using goldylox
pub struct FeaturesCache {
    cache: Goldylox<String, FeaturesValue>,
}

impl FeaturesCache {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxBuilder::<String, FeaturesValue>::new()
            .hot_tier_max_entries(300)
            .hot_tier_memory_limit_mb(16)
            .warm_tier_max_entries(1200)
            .warm_tier_max_memory_bytes(32 * 1024 * 1024) // 32MB
            .cold_tier_max_size_bytes(64 * 1024 * 1024) // 64MB
            .compression_level(5)
            .background_worker_threads(1)
            .cache_id("features_cache")
            .build()?;

        Ok(Self { cache })
    }

    pub fn get(&self, feature_name: &str, context: &str) -> Option<Vec<String>> {
        let key = format!("{}:{}", feature_name, context);
        self.cache.get(&key).map(|v| v.features)
    }

    pub fn put(
        &self,
        feature_name: String,
        context: String,
        features: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}:{}", feature_name, context);
        let value = FeaturesValue {
            features,
            cached_at: std::time::Instant::now(),
        };
        self.cache
            .put(key, value)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache
            .clear()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
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

impl Default for FeaturesCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback: create a minimal cache that always works
            FeaturesCache {
                cache: GoldyloxBuilder::<String, Vec<Feature>>::new()
                    .cache_id("features_cache_fallback")
                    .build()
                    .unwrap(),
            }
        })
    }
}
