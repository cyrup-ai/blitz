//! Cache integration for blitz-text
//!
//! This module provides cache integration for using goldylox high-performance cache system
//! with text processing types.

// Global singleton cache manager
pub mod global;

// Re-export goldylox types for convenience
pub use goldylox::traits::{CacheKey, CacheValue};
pub use goldylox::{Goldylox, GoldyloxBuilder};

// Re-export global cache functions
pub use global::{GlobalCacheManager, get_text_shaping_cache, get_text_measurement_cache, get_serialized_cache};

// Type alias for compatibility - generic cache manager
pub type CacheManager<K, V> = Goldylox<K, V>;

// Re-export measurement types for cache compatibility
pub use crate::measurement::types::CacheStats;
