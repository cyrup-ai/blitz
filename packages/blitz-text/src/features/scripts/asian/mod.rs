//! Asian script feature modules
//!
//! This module provides comprehensive OpenType feature support for Asian scripts
//! including Southeast Asian complex scripts, Mongolian vertical script, and CJK.

pub mod cjk;
pub mod common;
pub mod mongolian;
pub mod southeast_asian;

// Re-export all script creation functions
pub use cjk::{create_han_features, create_hangul_features, create_kana_features};
// Re-export common utilities for external use
pub use common::{BASE_FEATURES, STANDARD_SS_FEATURES, STANDARD_STYLISTIC_SETS};
pub use mongolian::create_mongolian_features;
pub use southeast_asian::{
    create_khmer_features, create_myanmar_features, create_tibetan_features,
};
