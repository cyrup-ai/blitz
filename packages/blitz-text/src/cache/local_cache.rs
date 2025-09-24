//! Local cache using goldylox multi-tier caching

use goldylox::{Goldylox, GoldyloxBuilder};
use goldylox::prelude::*;
use serde::{Serialize, Deserialize};

/// Generic local cache using goldylox with String keys
pub struct LocalCache<V> 
where 
    V: CacheValue + Clone + Serialize + for<'de> Deserialize<'de>,
{
    cache: Goldylox<String, V>,
    max_size: usize,
}

impl<V> LocalCache<V> 
where 
    V: CacheValue + Clone + Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(max_size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxBuilder::<String, V>::new()
            .hot_tier_max_entries(max_size / 4)
            .hot_tier_memory_limit_mb(32)
            .warm_tier_max_entries(max_size)
            .warm_tier_max_memory_bytes(64 * 1024 * 1024) // 64MB
            .cold_tier_max_size_bytes(128 * 1024 * 1024) // 128MB
            .compression_level(5)
            .background_worker_threads(1)
            .cache_id("local_cache")
            .build()?;
        
        Ok(Self { cache, max_size })
    }

    pub fn get(&self, key: &str) -> Option<V> {
        self.cache.get(key)
    }

    pub fn put(&mut self, key: String, value: V) -> Result<(), Box<dyn std::error::Error>> {
        self.cache.put(key, value).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache.clear().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn len(&self) -> usize {
        self.cache.len().unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }
}