//! Global feature settings registry with script-specific configurations

use std::collections::HashMap;

use once_cell::sync::Lazy;

use super::scripts::*;
use super::types::FeatureSettings;

/// Global feature settings registry with best-quality defaults
pub static FEATURE_REGISTRY: Lazy<HashMap<&'static str, FeatureSettings>> = Lazy::new(|| {
    let mut registry = HashMap::new();

    // Default settings with maximum typography quality enabled
    registry.insert(
        "default",
        FeatureSettings {
            ligatures: true,
            kerning: true,
            contextual_alternates: true,
            stylistic_sets: &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            ],
            opentype_features: &[
                ("kern", 1), // Kerning pairs
                ("liga", 1), // Standard ligatures
                ("clig", 1), // Contextual ligatures
                ("rlig", 1), // Required ligatures
                ("dlig", 1), // Discretionary ligatures
                ("hlig", 1), // Historical ligatures
                ("calt", 1), // Contextual alternates
                ("cswh", 1), // Contextual swash
                ("ccmp", 1), // Glyph composition/decomposition
                ("locl", 1), // Localized forms
                ("mark", 1), // Mark positioning
                ("mkmk", 1), // Mark-to-mark positioning
                ("frac", 1), // Fractions
                ("ordn", 1), // Ordinals
                ("sups", 1), // Superscripts
                ("subs", 1), // Subscripts
                ("smcp", 1), // Small capitals
                ("c2sc", 1), // Capitals to small capitals
                ("case", 1), // Case sensitive forms
                ("cpsp", 1), // Capital spacing
                ("swsh", 1), // Swash forms
                ("salt", 1), // Stylistic alternates
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
        },
    );

    // Arabic script with maximum quality enabled
    registry.insert("arab", create_arabic_features());

    // Devanagari script with maximum quality enabled
    registry.insert("deva", create_devanagari_features());

    // Bengali script with comprehensive features
    registry.insert("beng", create_bengali_features());

    // Thai script with advanced positioning
    registry.insert("thai", create_thai_features());

    // Hebrew script with complete feature set
    registry.insert("hebr", create_hebrew_features());

    // Myanmar script with comprehensive shaping
    registry.insert("mymr", create_myanmar_features());

    // Add all other scripts with maximum features
    registry.insert("khmr", create_khmer_features());
    registry.insert("tibt", create_tibetan_features());
    registry.insert("mong", create_mongolian_features());
    registry.insert("hang", create_hangul_features());
    registry.insert("hani", create_han_features());
    registry.insert("kana", create_kana_features());
    registry.insert("grek", create_greek_features());
    registry.insert("cyrl", create_cyrillic_features());

    registry
});
