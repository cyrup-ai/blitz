//! UAX #14 Unicode Line Breaking Algorithm Implementation
//!
//! This module provides a complete implementation of the Unicode Line Breaking Algorithm
//! as specified in UAX #14, with support for all line breaking classes, contextual rules,
//! complex scripts, and bidirectional text processing.
//!
//! The module is organized into focused submodules:
//! - `types`: Line breaking types, enums, and data structures
//! - `character_classification`: Unicode character classification with ASCII fast paths
//! - `rule_application`: UAX #14 pair table rules and break determination
//! - `analyzer`: Main analyzer logic and opportunity optimization

pub mod analyzer;
pub mod character_classification;
pub mod rule_application;
pub mod types;

// Re-export main public APIs
pub use analyzer::LineBreakAnalyzer;
pub use types::{BreakClass, BreakOpportunity, BreakPriority, CharacterExtensions, LineBreakClass};
