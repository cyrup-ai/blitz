//! BiDi cursor management module
//!
//! This module provides cursor positioning, hit testing, and selection management
//! for bidirectional text with proper visual-logical coordinate mapping.

mod core;
mod hit_tester;
mod position_calculator;
mod selection_manager;
mod types;

pub use core::CursorManager;

pub use hit_tester::{CharacterBoundaries, HitCoordinates, HitTestResult, HitTester};
pub use position_calculator::PositionCalculator;
pub use selection_manager::{SelectionManager, SelectionRectangle};
pub use types::CursorStats;
