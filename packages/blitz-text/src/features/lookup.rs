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

    /// Check if script uses cursive/contextual joining (compile-time constant)
    /// 
    /// Cursive scripts have letters that change form based on their position in a word
    /// (isolated, initial, medial, final). These scripts require special handling for
    /// glyph connection and shaping.
    /// 
    /// Covered scripts and their Unicode blocks:
    /// - Arabic (U+0600-U+06FF, U+0750-U+077F, U+08A0-U+08FF)
    /// - Syriac (U+0700-U+074F, U+0860-U+086F)
    /// - Mongolian (U+1800-U+18AF) - Traditional vertical script
    /// - Nko (U+07C0-U+07FF) - West African N'Ko
    /// - Mandaic (U+0840-U+085F) - Mandaean script
    /// - Phags_Pa (U+A840-U+A877) - Historical Phags-pa
    /// - Manichaean (U+10AC0-U+10AFF) - Historical Manichaean
    /// - Psalter_Pahlavi (U+10B80-U+10BAF) - Psalter Pahlavi
    /// - Hanifi_Rohingya (U+10D00-U+10D3F) - Hanifi Rohingya
    /// - Sogdian (U+10F30-U+10F6F) - Sogdian script
    /// - Old_Sogdian (U+10F00-U+10F2F) - Old Sogdian
    /// - Adlam (U+1E900-U+1E95F) - West African Adlam
    /// - Chorasmian (U+10FB0-U+10FCF) - Chorasmian
    /// - Elymaic (U+10FE0-U+10FFF) - Elymaic
    /// - Old_Uyghur (U+10F70-U+10FAF) - Old Uyghur
    #[inline]
    pub const fn is_cursive_script(script: Script) -> bool {
        matches!(
            script,
            Script::Adlam
                | Script::Arabic
                | Script::Chorasmian
                | Script::Elymaic
                | Script::Hanifi_Rohingya
                | Script::Mandaic
                | Script::Manichaean
                | Script::Mongolian
                | Script::Nko
                | Script::Old_Sogdian
                | Script::Old_Uyghur
                | Script::Phags_Pa
                | Script::Psalter_Pahlavi
                | Script::Sogdian
                | Script::Syriac
        )
    }
}
