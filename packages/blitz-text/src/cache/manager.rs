//! Cache manager using goldylox multi-tier caching

use crate::cache::local_cache::LocalCache;
use goldylox::prelude::*;
use serde::{Serialize, Deserialize};

/// Cache manager using goldylox with String keys
pub struct CacheManager<V> 
where 
    V: CacheValue + Clone + Serialize + for<'de> Deserialize<'de>,
{
    local_cache: LocalCache<String, V>,
}

impl<V> CacheManager<V>
where 
    V: CacheValue + Clone + Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(max_size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let local_cache = LocalCache::new(max_size)?;
        Ok(Self { local_cache })
    }

    pub fn get(&self, key: &str) -> Option<V> {
        self.local_cache.get(key)
    }

    pub fn put(&mut self, key: String, value: V) -> Result<(), Box<dyn std::error::Error>> {
        self.local_cache.put(key, value)
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.local_cache.clear()
    }

    pub fn len(&self) -> usize {
        self.local_cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.local_cache.is_empty()
    }

    pub fn max_size(&self) -> usize {
        self.local_cache.max_size()
    }
}