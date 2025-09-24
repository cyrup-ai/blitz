//! Southeast Asian script features (Myanmar, Khmer, Tibetan)
//!
//! This module implements OpenType features for complex Southeast Asian scripts
//! that require sophisticated glyph positioning and contextual shaping.

use super::common::STANDARD_STYLISTIC_SETS;
use crate::features::types::FeatureSettings;

/// Create Myanmar script features
pub const fn create_myanmar_features() -> FeatureSettings {
    const MYANMAR_FEATURES: &[(&str, u32)] = &[
        ("ccmp", 1),
        ("locl", 1),
        ("rlig", 1),
        ("liga", 1),
        ("clig", 1),
        ("dlig", 1),
        ("mark", 1),
        ("mkmk", 1),
        ("kern", 1),
        ("calt", 1),
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
        opentype_features: MYANMAR_FEATURES,
    }
}

/// Create Khmer script features
pub const fn create_khmer_features() -> FeatureSettings {
    const KHMER_FEATURES: &[(&str, u32)] = &[
        ("pref", 1),
        ("blwf", 1),
        ("abvf", 1),
        ("pstf", 1),
        ("cfar", 1),
        ("cjct", 1),
        ("mark", 1),
        ("mkmk", 1),
        ("kern", 1),
        ("liga", 1),
        ("ccmp", 1),
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
        opentype_features: KHMER_FEATURES,
    }
}

/// Create Tibetan script features
pub const fn create_tibetan_features() -> FeatureSettings {
    const TIBETAN_FEATURES: &[(&str, u32)] = &[
        ("ccmp", 1),
        ("abvs", 1),
        ("blws", 1),
        ("mark", 1),
        ("mkmk", 1),
        ("kern", 1),
        ("liga", 1),
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
        opentype_features: TIBETAN_FEATURES,
    }
}
