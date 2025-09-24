//! Unified Text System - High-level API combining measurement and GPU rendering
//!
//! This module provides a comprehensive, high-level text system that seamlessly
//! integrates cosmyc-text measurement capabilities with glyphon GPU rendering,
//! offering a complete solution for text layout, measurement, and rendering.

pub mod config;
pub mod core;
pub mod performance;

// Re-export public API from decomposed modules
pub use core::UnifiedTextSystem;

pub use config::{
    PreparedText, RenderMetrics, TextAreaConfig, TextSystemError, TextSystemResult,
    UnifiedTextConfig,
};
// Re-export types for convenience
pub use cosmyc_text::{
    Action, Align, Attrs, AttrsOwned, Buffer, BufferLine, CacheKeyFlags, Color, Cursor, Edit,
    Editor, Family, FamilyOwned, FontFeatures, FontSystem, LayoutRun, LineEnding, Metrics, Motion,
    Selection, Shaping, Style, Weight, Wrap,
};
pub use glyphon::{
    Cache, ColorMode, CustomGlyph, PrepareError, RenderError, Resolution, TextArea, TextBounds,
};
pub use performance::{ComprehensiveStats, SystemOptimizationResult, SystemPerformanceStats};

// Re-export cosmyc integration types
pub use crate::cosmyc::AttrsList;
pub use crate::measurement::types::CSSBaseline;
pub use crate::measurement::{
    CharacterPosition, LineMeasurement, MeasurementError, MeasurementResult, TextMeasurement,
};

// Tests extracted to tests/text_system_tests.rs for better performance
