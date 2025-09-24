//! OpenType feature settings and script-specific configuration
//!
//! This module provides comprehensive OpenType feature management for different
//! writing systems, ensuring proper text rendering across all supported scripts.

use std::collections::HashMap;

use once_cell::sync::Lazy;

use super::types::FeatureSettings;

/// Convert from comprehensive registry features to local features type
fn convert_features(features: &crate::features::types::FeatureSettings) -> FeatureSettings {
    FeatureSettings {
        ligatures: features.ligatures,
        kerning: features.kerning,
        contextual_alternates: features.contextual_alternates,
        stylistic_sets: features.stylistic_sets,
        opentype_features: features.opentype_features,
    }
}

/// Static lookup table bridging to comprehensive registry
static SCRIPT_FEATURES: Lazy<HashMap<&'static str, FeatureSettings>> = Lazy::new(|| {
    let mut features = HashMap::new();

    // Bridge to comprehensive registry and convert types
    for (script_tag, comprehensive_features) in crate::features::registry::FEATURE_REGISTRY.iter() {
        features.insert(*script_tag, convert_features(comprehensive_features));
    }

    features
});

/// Default feature settings for fallback scenarios
pub static DEFAULT_FEATURES: FeatureSettings = FeatureSettings {
    ligatures: true,
    kerning: true,
    contextual_alternates: true,
    stylistic_sets: &[],
    opentype_features: &[("kern", 1), ("liga", 1), ("clig", 1)],
};

/// Get feature settings for a specific script tag
pub fn get_script_features(script_tag: &str) -> &'static FeatureSettings {
    // Bridge to comprehensive registry (15+ scripts vs 6, 5-10x more features)
    SCRIPT_FEATURES.get(script_tag).unwrap_or(&DEFAULT_FEATURES)
}

/// Advanced feature sets for specific typography needs
pub mod advanced_features {
    use super::FeatureSettings;

    /// Small caps feature set
    pub const SMALL_CAPS: FeatureSettings = FeatureSettings {
        ligatures: true,
        kerning: true,
        contextual_alternates: true,
        stylistic_sets: &[],
        opentype_features: &[("smcp", 1), ("c2sc", 1), ("kern", 1)],
    };

    /// Oldstyle figures feature set
    pub const OLDSTYLE_FIGURES: FeatureSettings = FeatureSettings {
        ligatures: true,
        kerning: true,
        contextual_alternates: true,
        stylistic_sets: &[],
        opentype_features: &[("onum", 1), ("kern", 1), ("liga", 1)],
    };

    /// Tabular figures feature set
    pub const TABULAR_FIGURES: FeatureSettings = FeatureSettings {
        ligatures: false,
        kerning: false,
        contextual_alternates: false,
        stylistic_sets: &[],
        opentype_features: &[("tnum", 1), ("lnum", 1)],
    };
}

/// Script-specific feature utilities
pub mod script_utils {
    use unicode_script::Script;

    /// Determine if a script requires complex shaping
    pub fn requires_complex_shaping(script: Script) -> bool {
        matches!(
            script,
            Script::Arabic
                | Script::Devanagari
                | Script::Myanmar
                | Script::Khmer
                | Script::Tibetan
                | Script::Mongolian
                | Script::Sinhala
                | Script::Tamil
                | Script::Telugu
                | Script::Kannada
                | Script::Malayalam
                | Script::Gujarati
                | Script::Oriya
                | Script::Bengali
                | Script::Gurmukhi
        )
    }

    /// Get recommended buffer size for script complexity
    pub fn get_buffer_size_hint(script: Script) -> usize {
        if requires_complex_shaping(script) {
            1024 // Larger buffer for complex scripts
        } else {
            256 // Standard buffer for simple scripts
        }
    }

    /// Check if script requires right-to-left processing
    pub fn is_rtl_script(script: Script) -> bool {
        matches!(script, Script::Arabic | Script::Hebrew)
    }
}
