//! Enhanced text measurement using comprehensive cosmyc-text API integration
//!
//! This module provides advanced text measurement capabilities that leverage
//! all cosmyc-text APIs including cursor management, text editing, rich text,
//! and comprehensive layout analysis.

pub mod baseline_calculation;
pub mod bounds_calculation;
pub mod core;
pub mod font_metrics;
pub mod glyph_extraction;
pub mod hit_testing;
pub mod layout_analysis;
pub mod measurement;
pub mod types;

// Re-export public types and functionality
pub use core::EnhancedTextMeasurer;

pub use baseline_calculation::BaselineCalculator;
pub use bounds_calculation::BoundsCalculator;
pub use font_metrics::FontMetricsCalculator;
pub use glyph_extraction::GlyphExtractor;
pub use layout_analysis::LayoutAnalyzer;
pub use types::{BaselineInfo, EnhancedTextMeasurement};

// Tests extracted to tests/enhanced_measurement_tests.rs for better performance
