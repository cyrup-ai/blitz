//! Core feature types and data structures for OpenType feature management

/// OpenType feature settings for different scripts
#[derive(Debug, Clone)]
pub struct FeatureSettings {
    pub ligatures: bool,
    pub kerning: bool,
    pub contextual_alternates: bool,
    pub stylistic_sets: &'static [u8],
    pub opentype_features: &'static [(&'static str, u32)],
}

/// Static default feature settings with maximum typography quality
pub static DEFAULT_FEATURES: FeatureSettings = FeatureSettings {
    ligatures: true,
    kerning: true,
    contextual_alternates: true,
    stylistic_sets: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    opentype_features: &[
        ("kern", 1),
        ("liga", 1),
        ("clig", 1),
        ("rlig", 1),
        ("dlig", 1),
        ("hlig", 1),
        ("calt", 1),
        ("cswh", 1),
        ("ccmp", 1),
        ("locl", 1),
        ("mark", 1),
        ("mkmk", 1),
        ("frac", 1),
        ("ordn", 1),
        ("sups", 1),
        ("subs", 1),
        ("smcp", 1),
        ("c2sc", 1),
        ("case", 1),
        ("cpsp", 1),
        ("swsh", 1),
        ("salt", 1),
    ],
};

impl Default for FeatureSettings {
    fn default() -> Self {
        DEFAULT_FEATURES.clone()
    }
}

/// Custom feature configuration for advanced typography (allocation-free)
#[derive(Debug, Clone)]
pub struct CustomFeatures {
    pub features: ahash::AHashMap<&'static str, u32>,
    pub stylistic_sets: heapless::Vec<u8, 32>,
    pub character_variants: ahash::AHashMap<char, u8>,
}

impl Default for CustomFeatures {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomFeatures {
    /// Create new custom features configuration
    #[inline]
    pub fn new() -> Self {
        Self {
            features: ahash::AHashMap::new(),
            stylistic_sets: heapless::Vec::new(),
            character_variants: ahash::AHashMap::new(),
        }
    }

    /// Add OpenType feature with value (returns Result for allocation failures)
    #[inline]
    pub fn add_feature(
        &mut self,
        tag: &'static str,
        value: u32,
    ) -> Result<&mut Self, crate::error::ShapingError> {
        self.features.insert(tag, value);
        Ok(self)
    }

    /// Add stylistic set (returns Result for allocation failures)  
    #[inline]
    pub fn add_stylistic_set(&mut self, set: u8) -> Result<&mut Self, crate::error::ShapingError> {
        if !self.stylistic_sets.contains(&set) {
            self.stylistic_sets
                .push(set)
                .map_err(|_| crate::error::ShapingError::MemoryError)?;
        }
        Ok(self)
    }

    /// Add character variant (returns Result for allocation failures)
    #[inline]
    pub fn add_character_variant(
        &mut self,
        character: char,
        variant: u8,
    ) -> Result<&mut Self, crate::error::ShapingError> {
        self.character_variants.insert(character, variant);
        Ok(self)
    }
}
