//! Font metrics system - high-performance, zero-allocation font measurements
//!
//! This module provides comprehensive font metrics extraction and layout calculations
//! optimized for the Blitz browser engine's performance requirements.

pub mod core;
pub mod layout;

// Re-export main types for backward compatibility
pub use core::FontMetrics;

pub use layout::FontLayoutMetrics;
