//! CJK (Chinese, Japanese, Korean) script features
//!
//! This module implements OpenType features for Han characters, Hangul,
//! and Kana scripts used across East Asian languages.

use super::common::STANDARD_STYLISTIC_SETS;
use crate::features::types::FeatureSettings;

/// Create Hangul script features
pub const fn create_hangul_features() -> FeatureSettings {
    const HANGUL_FEATURES: &[(&str, u32)] = &[
        ("ccmp", 1),
        ("ljmo", 1),
        ("vjmo", 1),
        ("tjmo", 1),
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
        opentype_features: HANGUL_FEATURES,
    }
}

/// Create Han script features
pub const fn create_han_features() -> FeatureSettings {
    const HAN_FEATURES: &[(&str, u32)] = &[
        ("kern", 1),
        ("liga", 1),
        ("calt", 1),
        ("locl", 1),
        ("trad", 1),
        ("smpl", 1),
        ("jp78", 1),
        ("jp83", 1),
        ("jp90", 1),
        ("jp04", 1),
        ("nlck", 1),
        ("ruby", 1),
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
        opentype_features: HAN_FEATURES,
    }
}

/// Create Kana script features
pub const fn create_kana_features() -> FeatureSettings {
    const KANA_FEATURES: &[(&str, u32)] = &[
        ("kern", 1),
        ("liga", 1),
        ("calt", 1),
        ("locl", 1),
        ("jp78", 1),
        ("jp83", 1),
        ("jp90", 1),
        ("jp04", 1),
        ("ruby", 1),
        ("hkna", 1),
        ("vkna", 1),
        ("pkna", 1),
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
        opentype_features: KANA_FEATURES,
    }
}
