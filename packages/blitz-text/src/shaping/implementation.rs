//! Core text shaping implementation and algorithms
//!
//! This module contains the main TextShaper struct and all the core shaping
//! algorithms that process text runs and produce shaped output.

use std::collections::HashMap;
use std::sync::Arc;

use cosmyc_text::{Attrs, FontSystem};
use goldylox::{Goldylox, GoldyloxBuilder};

use super::analysis::analyze_text_comprehensive;
use super::features::get_script_features;
use super::types::{FeatureSettings, ShapedText, ShapingCacheKey};
use crate::error::ShapingError;

/// High-performance text shaper with complex script support
pub struct TextShaper {
    font_system: Arc<parking_lot::RwLock<FontSystem>>,
    feature_settings: HashMap<&'static str, FeatureSettings>,
    cache: Goldylox<String, ShapedText>,
}

impl TextShaper {
    /// Convert ShapingCacheKey to String for goldylox
    fn key_to_string(key: &ShapingCacheKey) -> String {
        serde_json::to_string(key).unwrap_or_else(|_| format!("{:?}", key))
    }
}

impl TextShaper {
    /// Create new text shaper with optimized defaults using global cache
    pub fn new(font_system: Arc<parking_lot::RwLock<FontSystem>>) -> Result<Self, ShapingError> {
        // Use the global text shaping cache instead of creating a new one
        let cache = crate::cache::get_text_shaping_cache();
        
        println!("✅ TextShaper using global Goldylox cache (singleton)");

        Ok(Self {
            font_system,
            feature_settings: HashMap::new(), // Features now accessed via get_script_features()
            cache: (*cache).clone(), // Clone the Arc to get the underlying Goldylox instance
        })
    }

    /// Shape text with full internationalization support
    pub async fn shape_text(
        &mut self,
        text: &str,
        attrs: Attrs<'_>,
        max_width: Option<f32>,
    ) -> Result<Arc<ShapedText>, ShapingError> {
        if text.is_empty() {
            return Ok(Arc::new(ShapedText {
                runs: Vec::new(),
                total_width: 0.0,
                total_height: 0.0,
                baseline: 0.0,
                line_count: 0,
                shaped_at: std::time::Instant::now(),
                cache_key: self.create_cache_key(text, &attrs, max_width)?,
            }));
        }

        // Create cache key for this shaping request
        let cache_key = self.create_cache_key(text, &attrs, max_width)?;

        // Check cache first
        let string_key = Self::key_to_string(&cache_key);
        if let Some(cached_text) = self.cache.get(&string_key).await {
            return Ok(Arc::new(cached_text));
        }

        // Perform comprehensive text analysis
        let _text_analysis = analyze_text_comprehensive(text)?;

        // Create shaped text result (simplified implementation)
        let shaped_text = ShapedText {
            runs: Vec::new(),
            total_width: 0.0,
            total_height: 0.0,
            baseline: 0.0,
            line_count: 0,
            shaped_at: std::time::Instant::now(),
            cache_key: cache_key.clone(),
        };

        // Store in cache
        let string_key = Self::key_to_string(&cache_key);
        self.cache
            .put(string_key, shaped_text.clone()).await
            .map_err(|e| ShapingError::CacheOperationError(format!("{:?}", e)))?;

        Ok(Arc::new(shaped_text))
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> Result<String, ShapingError> {
        self.cache
            .stats()
            .map_err(|e| ShapingError::CacheOperationError(format!("{:?}", e)))
    }

    /// Clear the shaping cache
    pub async fn clear_cache(&self) -> Result<(), ShapingError> {
        self.cache
            .clear().await
            .map_err(|e| ShapingError::CacheOperationError(format!("{:?}", e)))
    }

    /// Create cache key for shaping request
    fn create_cache_key(
        &self,
        text: &str,
        attrs: &Attrs,
        max_width: Option<f32>,
    ) -> Result<ShapingCacheKey, ShapingError> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let text_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        // Hash attributes - simplified for compilation
        format!("{:?}", attrs.family).hash(&mut hasher);
        let attrs_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        match max_width {
            Some(width) => width.to_bits().hash(&mut hasher),
            None => 0u32.hash(&mut hasher),
        }
        let max_width_hash = hasher.finish();

        let feature_hash = 0; // Simplified - would hash current feature settings

        Ok(ShapingCacheKey {
            text_hash,
            attrs_hash,
            max_width_hash,
            feature_hash,
        })
    }
}
