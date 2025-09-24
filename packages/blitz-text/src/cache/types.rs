//! Cache types - simplified implementation
//! Core caching is now handled by goldylox in the text shaping layer

use crate::cache::operations::{CacheOperation, CacheResult};

/// Cache error types - simplified
#[derive(Debug, Clone)]
pub enum CacheOperationError {
    NotFound,
    StorageFull,
    InvalidKey,
    SerializationError(String),
    Other(String),
}

impl std::fmt::Display for CacheOperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheOperationError::NotFound => write!(f, "Cache entry not found"),
            CacheOperationError::StorageFull => write!(f, "Cache storage is full"),
            CacheOperationError::InvalidKey => write!(f, "Invalid cache key"),
            CacheOperationError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            CacheOperationError::Other(msg) => write!(f, "Cache error: {}", msg),
        }
    }
}

impl std::error::Error for CacheOperationError {}

/// Hit status for cache operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitStatus {
    Hit,
    Miss,
    Error,
}

/// Cache configuration - simplified
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub max_size_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_size_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}