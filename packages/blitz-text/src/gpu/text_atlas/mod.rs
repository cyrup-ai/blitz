//! Enhanced TextAtlas module
//!
//! This module provides a comprehensive GPU texture atlas implementation using glyphon's
//! TextAtlas with enhanced performance monitoring, memory management, and
//! intelligent caching strategies.

pub mod core;
pub mod memory_management;
pub mod optimization;
pub mod statistics;
pub mod types;

// Re-export main types for convenience
pub use core::EnhancedTextAtlas;

// Re-export cosmyc-text types
pub use cosmyc_text::{FontSystem, SwashCache};
// Re-export glyphon types for compatibility
pub use glyphon::{
    Cache, ColorMode, ContentType, RasterizeCustomGlyphRequest, RasterizedCustomGlyph, TextAtlas,
};
pub use types::{
    AtlasGrowthEvent, AtlasStats, GrowthPrediction, MemoryBreakdown, OptimizationResult, TrimEvent,
};
