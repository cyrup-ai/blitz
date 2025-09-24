//! Core unified text system implementation
//!
//! This module contains the main UnifiedTextSystem struct and its
//! core functionality for text measurement, preparation, and rendering.
//!
//! The implementation is decomposed into logical modules:
//! - `system`: Core struct definition and constructors
//! - `preparation`: Text preparation and buffer management  
//! - `rendering`: Text rendering operations
//! - `management`: System management, optimization, and configuration

// Module declarations - only include the original intended modules
pub mod management;
pub mod measurement_ops;
pub mod preparation;
pub mod rendering;
pub mod system;

// Re-export the main struct and key types for API compatibility
pub use system::UnifiedTextSystem;
