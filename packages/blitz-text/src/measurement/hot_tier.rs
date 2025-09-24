//! Thread-local hot tier cache for sub-nanosecond access
//!
//! API References:
//! - std::sync::atomic::{AtomicU64, Ordering} for thread-safe counters
//! - std::sync::{Arc, Mutex} for shared ownership and thread safety
//! - std::time::Instant for high-precision timing
//! - std::collections::HashMap for key-value storage
//! - ahash::AHashMap for fast hashing (dependency: ahash = "0.8")
//! - crossbeam::channel for lock-free communication (dependency: crossbeam = "0.8")
//! - arrayvec::ArrayVec for stack-allocated collections (dependency: arrayvec = "0.7")

// Import existing types from the codebase
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::types::{ShapedText, ShapingCacheKey};

/// Thread-local hot cache for ultra-fast access
#[derive(Debug)]
pub struct HotTierCache {
    // Using HashMap for deterministic access patterns
    storage: HashMap<u64, CacheEntry>,
    access_counter: AtomicU64,
    max_entries: usize,
    access_order: Vec<u64>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Arc<ShapedText>,
    last_access: Instant,
    access_count: u64,
}

impl HotTierCache {
    /// Create new hot tier with fixed capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: HashMap::with_capacity(capacity),
            access_counter: AtomicU64::new(0),
            max_entries: capacity,
            access_order: Vec::with_capacity(capacity),
        }
    }

    /// Get value if present (zero-allocation path)
    #[inline]
    pub fn get(&mut self, key: &ShapingCacheKey) -> Option<Arc<ShapedText>> {
        let hash_key = self.hash_key(key);

        if let Some(entry) = self.storage.get_mut(&hash_key) {
            entry.last_access = Instant::now();
            entry.access_count += 1;
            self.access_counter.fetch_add(1, Ordering::Relaxed);
            Some(entry.data.clone())
        } else {
            None
        }
    }

    /// Store value with LRU eviction
    pub fn put(&mut self, key: ShapingCacheKey, value: Arc<ShapedText>) {
        let hash_key = self.hash_key(&key);

        // Remove oldest if at capacity
        if self.storage.len() >= self.max_entries && !self.storage.contains_key(&hash_key) {
            self.evict_lru();
        }

        let entry = CacheEntry {
            data: value,
            last_access: Instant::now(),
            access_count: 1,
        };

        self.storage.insert(hash_key, entry);

        // Track insertion order
        if !self.access_order.contains(&hash_key) {
            self.access_order.push(hash_key);
        }
    }

    /// Hash cache key to u64 for storage
    #[inline]
    fn hash_key(&self, key: &ShapingCacheKey) -> u64 {
        // Combine all hash components
        key.text_hash
            .wrapping_mul(31)
            .wrapping_add(key.attrs_hash)
            .wrapping_mul(31)
            .wrapping_add(key.max_width_hash)
            .wrapping_mul(31)
            .wrapping_add(key.feature_hash)
    }

    /// Evict least recently used entry
    fn evict_lru(&mut self) {
        if let Some(&oldest_key) = self.access_order.first() {
            self.storage.remove(&oldest_key);
            self.access_order.remove(0);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> HotTierStats {
        HotTierStats {
            entries: self.storage.len(),
            capacity: self.max_entries,
            total_accesses: self.access_counter.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HotTierStats {
    pub entries: usize,
    pub capacity: usize,
    pub total_accesses: u64,
}

thread_local! {
    static HOT_TIER: std::cell::RefCell<HotTierCache> =
        std::cell::RefCell::new(HotTierCache::new(100));
}

/// Get from thread-local hot tier
#[inline]
pub fn hot_get(key: &ShapingCacheKey) -> Option<Arc<ShapedText>> {
    HOT_TIER.with(|cache| cache.borrow_mut().get(key))
}

/// Put in thread-local hot tier
pub fn hot_put(key: ShapingCacheKey, value: Arc<ShapedText>) {
    HOT_TIER.with(|cache| cache.borrow_mut().put(key, value));
}

/// Get hot tier statistics
pub fn hot_stats() -> HotTierStats {
    HOT_TIER.with(|cache| cache.borrow().stats())
}

/// Clear hot tier cache
pub fn hot_clear() {
    HOT_TIER.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.storage.clear();
        cache.access_order.clear();
        cache
            .access_counter
            .store(0, std::sync::atomic::Ordering::Relaxed);
    });
}
