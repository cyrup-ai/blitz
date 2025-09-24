//! Cache operations using goldylox multi-tier caching

use goldylox::traits::{CacheKey, CacheValue};
use serde::{Serialize, Deserialize};

/// Cache operations for goldylox
pub enum CacheOperation<K, V> 
where
    K: CacheKey,
    V: CacheValue,
{
    Get(K),
    Put(K, V),
    Clear,
    Remove(K),
    Contains(K),
}

/// Cache operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheResult<V> 
where
    V: CacheValue,
{
    Hit(V),
    Miss,
    Success,
    Error(String),
    Contains(bool),
}

impl<V> CacheResult<V> 
where
    V: CacheValue,
{
    pub fn is_hit(&self) -> bool {
        matches!(self, CacheResult::Hit(_))
    }

    pub fn is_miss(&self) -> bool {
        matches!(self, CacheResult::Miss)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, CacheResult::Success)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, CacheResult::Error(_))
    }

    pub fn unwrap_hit(self) -> V {
        match self {
            CacheResult::Hit(value) => value,
            _ => panic!("Called unwrap_hit on non-hit result"),
        }
    }

    pub fn unwrap_contains(self) -> bool {
        match self {
            CacheResult::Contains(contains) => contains,
            _ => panic!("Called unwrap_contains on non-contains result"),
        }
    }
}

/// Cache statistics for goldylox operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub total_operations: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub puts: u64,
    pub removes: u64,
    pub clears: u64,
}

impl CacheStatistics {
    pub fn hit_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_operations as f64
        }
    }

    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    pub fn record_hit(&mut self) {
        self.total_operations += 1;
        self.cache_hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.total_operations += 1;
        self.cache_misses += 1;
    }

    pub fn record_put(&mut self) {
        self.puts += 1;
    }

    pub fn record_remove(&mut self) {
        self.removes += 1;
    }

    pub fn record_clear(&mut self) {
        self.clears += 1;
    }
}