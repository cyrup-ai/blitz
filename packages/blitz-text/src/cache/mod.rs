//! Cache integration for blitz-text
//!
//! This module provides cache integration for using goldylox high-performance cache system
//! with text processing types.

// Re-export goldylox types for convenience
pub use goldylox::traits::{CacheKey, CacheValue};
pub use goldylox::{Goldylox, GoldyloxBuilder};

// Type alias for compatibility - generic cache manager
pub type CacheManager<K, V> = Goldylox<K, V>;

// Re-export measurement types for cache compatibility
pub use crate::measurement::types::CacheStats;
