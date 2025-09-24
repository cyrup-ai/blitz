//! Zero-allocation feature lookup with compile-time optimization

use unicode_script::Script;

use super::registry::FEATURE_REGISTRY;
use super::types::{FeatureSettings, DEFAULT_FEATURES};

/// Zero-allocation feature lookup with compile-time optimization
pub struct FeatureLookup;

impl FeatureLookup {
    /// Get feature settings for a script with zero allocation
    #[inline]
    pub fn get_features_for_script(script: Script) -> &'static FeatureSettings {
        let script_tag = Self::script_to_tag(script);
        FEATURE_REGISTRY
            .get(script_tag)
            .unwrap_or(&DEFAULT_FEATURES)
    }

    /// Get feature settings by script tag (zero allocation)
    #[inline]
    pub fn get_features_by_tag(tag: &str) -> &'static FeatureSettings {
        FEATURE_REGISTRY.get(tag).unwrap_or(&DEFAULT_FEATURES)
    }

    /// Convert Unicode script to OpenType script tag (zero allocation)
    #[inline]
    pub const fn script_to_tag(script: Script) -> &'static str {
        match script {
            Script::Arabic => "arab",
            Script::Armenian => "armn",
            Script::Bengali => "beng",
            Script::Cyrillic => "cyrl",
            Script::Devanagari => "deva",
            Script::Georgian => "geor",
            Script::Greek => "grek",
            Script::Gujarati => "gujr",
            Script::Gurmukhi => "guru",
            Script::Hangul => "hang",
            Script::Han => "hani",
            Script::Hebrew => "hebr",
            Script::Hiragana => "kana",
            Script::Katakana => "kana",
            Script::Kannada => "knda",
            Script::Khmer => "khmr",
            Script::Lao => "lao ",
            Script::Latin => "default",
            Script::Malayalam => "mlym",
            Script::Mongolian => "mong",
            Script::Myanmar => "mymr",
            Script::Oriya => "orya",
            Script::Sinhala => "sinh",
            Script::Tamil => "taml",
            Script::Telugu => "telu",
            Script::Thai => "thai",
            Script::Tibetan => "tibt",
            _ => "default",
        }
    }

    /// Check if script requires complex shaping (compile-time constant)
    #[inline]
    pub const fn requires_complex_shaping(script: Script) -> bool {
        matches!(
            script,
            Script::Arabic
                | Script::Devanagari
                | Script::Bengali
                | Script::Gurmukhi
                | Script::Gujarati
                | Script::Oriya
                | Script::Tamil
                | Script::Telugu
                | Script::Kannada
                | Script::Malayalam
                | Script::Sinhala
                | Script::Myanmar
                | Script::Khmer
                | Script::Tibetan
                | Script::Mongolian
        )
    }

    /// Check if script requires bidirectional processing (compile-time constant)
    #[inline]
    pub const fn requires_bidi_processing(script: Script) -> bool {
        matches!(script, Script::Arabic | Script::Hebrew)
    }

    /// Get complexity level for caching priority (compile-time constant)
    #[inline]
    pub const fn get_complexity_level(script: Script) -> u8 {
        match script {
            Script::Latin | Script::Cyrillic | Script::Greek => 1,
            Script::Hebrew | Script::Thai | Script::Lao => 2,
            Script::Arabic
            | Script::Devanagari
            | Script::Bengali
            | Script::Myanmar
            | Script::Tamil => 3,
            Script::Khmer | Script::Tibetan | Script::Mongolian => 4,
            _ => 1,
        }
    }

    /// Check if script is right-to-left (compile-time constant)
    #[inline]
    pub const fn is_rtl_script(script: Script) -> bool {
        matches!(script, Script::Arabic | Script::Hebrew)
    }

    /// Check if script requires mark positioning (compile-time constant)
    #[inline]
    pub const fn requires_mark_positioning(script: Script) -> bool {
        !matches!(script, Script::Latin | Script::Cyrillic | Script::Greek)
    }
}
