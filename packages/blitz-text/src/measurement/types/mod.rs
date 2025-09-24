//! Type definitions for text measurement system
//!
//! This module contains all data structures and type definitions used throughout
//! the text measurement system, ensuring clean separation of concerns.

pub mod baseline_types;
pub mod bounds_types;
pub mod cache_impl;
pub mod cache_keys;
pub mod cache_types;
pub mod errors;
pub mod measurement_request;
pub mod measurement_results;
pub mod statistics;

// Re-export all public types for convenience
pub use baseline_types::{CSSBaseline, FontMetrics};
pub use bounds_types::{CharacterPosition, InkBounds, LogicalBounds, TextBounds};
pub use cache_impl::MeasurementCache;
pub use cache_keys::{BaselineCacheKey, FontMetricsCacheKey, MeasurementCacheKey};
pub use cache_types::{CacheStats, ShapedText, ShapingCacheKey};
pub use errors::{MeasurementError, MeasurementResult};
pub use measurement_request::{MeasurementRequest, TextDirection};
pub use measurement_results::{LineMeasurement, TextMeasurement};
pub use statistics::{MeasurementStats, MeasurementStatsInner};
