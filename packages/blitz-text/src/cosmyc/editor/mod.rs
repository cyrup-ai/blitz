//! Enhanced Editor integration with comprehensive cosmyc-text editing capabilities
//!
//! This module provides complete integration with cosmyc-text's Editor for text editing
//! operations including cursor management, selections, and undo/redo functionality.

pub mod actions;
pub mod edit_operations;
pub mod statistics;
pub mod types;

// Re-export main types and structs
pub use types::{EditorStats, EnhancedEditor};

// Tests extracted to tests/cosmyc_editor_tests.rs for better performance
