//! Feature creation functions for European scripts and alphabets

use crate::features::types::FeatureSettings;

/// Create Greek script features
pub const fn create_greek_features() -> FeatureSettings {
    FeatureSettings {
        ligatures: true,
        kerning: true,
        contextual_alternates: true,
        stylistic_sets: &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ],
        opentype_features: &[
            ("kern", 1),
            ("liga", 1),
            ("dlig", 1),
            ("hlig", 1),
            ("calt", 1),
            ("ccmp", 1),
            ("locl", 1),
            ("mark", 1),
            ("mkmk", 1),
            ("smcp", 1),
            ("c2sc", 1),
            ("case", 1),
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
        ],
    }
}

/// Create Cyrillic script features
pub const fn create_cyrillic_features() -> FeatureSettings {
    FeatureSettings {
        ligatures: true,
        kerning: true,
        contextual_alternates: true,
        stylistic_sets: &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ],
        opentype_features: &[
            ("kern", 1),
            ("liga", 1),
            ("dlig", 1),
            ("hlig", 1),
            ("calt", 1),
            ("ccmp", 1),
            ("locl", 1),
            ("mark", 1),
            ("mkmk", 1),
            ("smcp", 1),
            ("c2sc", 1),
            ("case", 1),
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
        ],
    }
}
