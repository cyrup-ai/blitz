//! Mongolian script features for traditional vertical writing
//!
//! This module implements OpenType features for the Mongolian script,
//! which has unique vertical writing characteristics and contextual forms.

use super::common::STANDARD_STYLISTIC_SETS;
use crate::features::types::FeatureSettings;

/// Create Mongolian script features (vertical)
pub const fn create_mongolian_features() -> FeatureSettings {
    const MONGOLIAN_FEATURES: &[(&str, u32)] = &[
        ("init", 1),
        ("medi", 1),
        ("fina", 1),
        ("isol", 1),
        ("ccmp", 1),
        ("liga", 1),
        ("kern", 1),
        ("calt", 1),
        ("locl", 1),
        ("ss01", 1),
        ("ss02", 1),
        ("ss03", 1),
        ("ss04", 1),
        ("ss05", 1),
        ("ss06", 1),
        ("ss07", 1),
        ("ss08", 1),
        ("ss09", 1),
        ("ss10", 1),
        ("ss11", 1),
        ("ss12", 1),
        ("ss13", 1),
        ("ss14", 1),
        ("ss15", 1),
        ("ss16", 1),
        ("ss17", 1),
        ("ss18", 1),
        ("ss19", 1),
        ("ss20", 1),
    ];

    FeatureSettings {
        ligatures: true,
        kerning: true,
        contextual_alternates: true,
        stylistic_sets: STANDARD_STYLISTIC_SETS,
        opentype_features: MONGOLIAN_FEATURES,
    }
}
