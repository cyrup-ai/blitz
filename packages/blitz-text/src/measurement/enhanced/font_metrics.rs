//! Font metrics using goldylox multi-tier caching

use cosmyc_text::fontdb;
use goldylox::prelude::*;
use serde::{Deserialize, Serialize};

use crate::measurement::types::FontMetrics;

/// Font metrics cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct FontMetricsCacheKey {
    pub font_id: u32, // Using u32 instead of fontdb::ID for serialization
    pub font_size_bits: u32,
}

impl FontMetricsCacheKey {
    pub fn new(font_id: fontdb::ID, font_size: f32) -> Self {
        // Since fontdb::ID constructor is private, we use a hash of the ID as a workaround
        // This maintains cache functionality while avoiding private field access
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        font_id.hash(&mut hasher);
        Self {
            font_id: hasher.finish() as u32,
            font_size_bits: font_size.to_bits(),
        }
    }
}

impl CacheKey for FontMetricsCacheKey {
    type HashContext = StandardHashContext;
    type Priority = StandardPriority;
    type SizeEstimator = StandardSizeEstimator;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn hash_context(&self) -> Self::HashContext {
        use goldylox::cache::traits::supporting_types::HashAlgorithm;
        StandardHashContext::new(HashAlgorithm::AHash, 0x517cc1b727220a95)
    }

    fn fast_hash(&self, _context: &Self::HashContext) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn priority(&self) -> Self::Priority {
        // Font metrics priority based on font size
        let font_size = f32::from_bits(self.font_size_bits);
        let priority_value = if font_size > 32.0 {
            6 // Medium priority for large fonts
        } else if font_size < 8.0 {
            3 // Lower priority for very small fonts
        } else {
            9 // High priority for normal reading sizes
        };
        StandardPriority::new(priority_value)
    }

    fn size_estimator(&self) -> Self::SizeEstimator {
        StandardSizeEstimator::new()
    }
}

impl CacheValue for FontMetrics {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn is_expensive(&self) -> bool {
        false // Font metrics are lightweight
    }

    fn compression_hint(&self) -> CompressionHint {
        CompressionHint::Auto
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

/// Font metrics cache using goldylox
pub struct FontMetricsCache {
    cache: Goldylox<FontMetricsCacheKey, FontMetrics>,
}

impl FontMetricsCache {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache = GoldyloxBuilder::new()
            .hot_tier_max_entries(200)
            .hot_tier_memory_limit_mb(16)
            .warm_tier_max_entries(800)
            .warm_tier_max_memory_bytes(32 * 1024 * 1024) // 32MB
            .cold_tier_max_size_bytes(64 * 1024 * 1024) // 64MB
            .compression_level(5)
            .background_worker_threads(1)
            .cache_id("font_metrics_cache")
            .build()?;

        Ok(Self { cache })
    }

    pub fn get(&self, font_id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        let key = FontMetricsCacheKey::new(font_id, font_size);
        self.cache.get(&key)
    }

    pub fn put(
        &self,
        font_id: fontdb::ID,
        font_size: f32,
        metrics: FontMetrics,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = FontMetricsCacheKey::new(font_id, font_size);
        self.cache
            .put(key, metrics)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache
            .clear()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub fn len(&self) -> usize {
        // Goldylox doesn't expose len() - return 0 as placeholder
        0
    }

    pub fn is_empty(&self) -> bool {
        // Goldylox doesn't expose len() - return true as placeholder
        true
    }
}

impl Default for FontMetricsCache {
    fn default() -> Self {
        Self::new().expect("Failed to create FontMetricsCache")
    }
}

/// Font metrics calculator using goldylox caching
pub struct FontMetricsCalculator {
    cache: FontMetricsCache,
}

impl FontMetricsCalculator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache = FontMetricsCache::new()?;
        Ok(Self { cache })
    }

    pub fn calculate(&self, font_id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        if let Some(cached) = self.cache.get(font_id, font_size) {
            return Some(cached);
        }

        // Calculate font metrics - simplified implementation
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: (font_size * 0.8) as i16,
            descent: -(font_size * 0.2) as i16,
            line_gap: 0,
            x_height: Some((font_size * 0.5) as i16),
            cap_height: Some((font_size * 0.7) as i16),
            ideographic_baseline: Some(-(font_size * 0.1) as i16),
            hanging_baseline: Some((font_size * 0.8) as i16),
            mathematical_baseline: Some((font_size * 0.4) as i16),
            average_char_width: font_size * 0.6,
            max_char_width: font_size * 1.2,
            underline_position: font_size * -0.1,
            underline_thickness: font_size * 0.05,
            strikethrough_position: font_size * 0.4,
            strikethrough_thickness: font_size * 0.05,
        };

        if let Err(e) = self.cache.put(font_id, font_size, metrics.clone()) {
            eprintln!("Warning: Failed to cache font metrics: {}", e);
        }

        Some(metrics)
    }

    pub fn get_cached(&self, font_id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        self.cache.get(font_id, font_size)
    }

    pub fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache.clear()
    }

    /// Extract comprehensive font metrics for given attributes
    pub fn extract_comprehensive_font_metrics(
        &self,
        _attrs: &cosmyc_text::Attrs,
        _font_system: &mut cosmyc_text::FontSystem,
    ) -> Result<FontMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // Return default font metrics for now since we can't access fontdb::ID constructor
        // TODO: Implement proper font ID retrieval from FontSystem once API is clarified
        Ok(FontMetrics::default())
    }
}

impl Default for FontMetricsCalculator {
    fn default() -> Self {
        Self::new().expect("Failed to create FontMetricsCalculator")
    }
}
