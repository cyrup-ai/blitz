//! Public API interface for text measurement operations
//!
//! This module provides the unified public interface that re-exports
//! all public functions from the decomposed measurement modules.

// Re-export all public functions from decomposed modules
pub use super::font_metrics::{calculate_baseline_offset, get_baseline_offset, get_font_metrics};
pub use super::glyph_processing::{
    extract_physical_glyphs, get_text_highlight_bounds, measure_layout_run_enhanced,
};
pub use super::text_measurement::{get_character_positions, perform_measurement};
// Re-export types for convenience
pub use super::types::*;
