//! Multi-line BiDi text processing
//!
//! This module handles line breaking, paragraph processing, and multi-line
//! bidirectional text layout with proper line wrapping and paragraph handling.

mod core;
mod line_breaking;
mod statistics;
mod text_wrapping;

pub use core::MultiLineBidiProcessor;

pub use line_breaking::LineBreaker;
pub use statistics::{LineDistribution, MultiLineOptimizer, MultiLineStats};
pub use text_wrapping::TextWrapper;
