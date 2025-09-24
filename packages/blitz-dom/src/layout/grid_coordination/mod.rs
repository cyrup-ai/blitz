//! CSS Grid Multi-Pass Layout Coordination System
//!
//! Implements the complete multi-pass layout coordination system for CSS Grid Level 2 subgrid
//! and CSS Grid Level 3 masonry layout support.

pub mod coordinator;
pub mod helpers;
pub mod placement;
pub mod placement_types;
pub mod track_types;
pub mod types;

// Re-export all public types from the modules
pub use placement_types::*;
pub use track_types::*;
pub use types::*;

// The implementation methods are included via the modules
// but the types are the main interface
