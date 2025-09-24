//! Enhanced measurement core using goldylox multi-tier caching

use cosmyc_text::fontdb;
use goldylox::traits::CacheKey;
use goldylox::{Goldylox, GoldyloxBuilder};
use serde::{Deserialize, Serialize};

use crate::measurement::types::*;

/// Enhanced measurement cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct EnhancedMeasurementKey {
    pub text_hash: u64,
    pub font_id: u32,
    pub font_size_bits: u32,
    pub width_bits: u32,
}

impl EnhancedMeasurementKey {
    pub fn new(text: &str, font_id: u32, font_size: f32, width: Option<f32>) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut text_hasher = DefaultHasher::new();
        text.hash(&mut text_hasher);

        Self {
            text_hash: text_hasher.finish(),
            font_id,
            font_size_bits: font_size.to_bits(),
            width_bits: width.unwrap_or(f32::INFINITY).to_bits(),
        }
    }
}

impl CacheKey for EnhancedMeasurementKey {
    type HashContext = goldylox::prelude::StandardHashContext;
    type Priority = goldylox::prelude::StandardPriority;
    type SizeEstimator = goldylox::prelude::StandardSizeEstimator;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn hash_context(&self) -> Self::HashContext {
        use goldylox::cache::traits::supporting_types::HashAlgorithm;
        goldylox::prelude::StandardHashContext::new(HashAlgorithm::AHash, 0x517cc1b727220a95)
    }

    fn fast_hash(&self, _context: &Self::HashContext) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn priority(&self) -> Self::Priority {
        // Enhanced measurement priority based on text size and font characteristics
        let priority_value = if self.font_size_bits > (24.0_f32).to_bits() {
            7 // Medium-high priority for large fonts
        } else if self.font_size_bits < (10.0_f32).to_bits() {
            4 // Lower priority for very small fonts
        } else {
            8 // High priority for normal reading sizes
        };
        goldylox::prelude::StandardPriority::new(priority_value)
    }

    fn size_estimator(&self) -> Self::SizeEstimator {
        goldylox::prelude::StandardSizeEstimator::new()
    }
}

/// Enhanced measurement system using goldylox
pub struct EnhancedMeasurementCore {
    cache: Goldylox<String, TextMeasurement>,
}

impl EnhancedMeasurementCore {
    /// Convert EnhancedMeasurementKey to String for goldylox
    fn key_to_string(key: &EnhancedMeasurementKey) -> String {
        serde_json::to_string(key).unwrap_or_else(|_| format!("{:?}", key))
    }
}

impl EnhancedMeasurementCore {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxBuilder::<String, TextMeasurement>::new()
            .hot_tier_max_entries(1000)
            .hot_tier_memory_limit_mb(64)
            .warm_tier_max_entries(4000)
            .warm_tier_max_memory_bytes(128 * 1024 * 1024) // 128MB
            .cold_tier_max_size_bytes(256 * 1024 * 1024) // 256MB
            .compression_level(6)
            .background_worker_threads(2)
            .cache_id("enhanced_measurement_core")
            .build()?;

        Ok(Self { cache })
    }

    pub fn measure_text(
        &mut self,
        request: &MeasurementRequest,
    ) -> Result<TextMeasurement, MeasurementError> {
        let key = EnhancedMeasurementKey::new(
            &request.text,
            request.font_id,
            request.font_size,
            request.max_width,
        );
        let string_key = Self::key_to_string(&key);

        if let Some(cached) = self.cache.get(&string_key) {
            return Ok(cached);
        }

        // Simplified measurement - real implementation would use proper text shaping
        let measurement = TextMeasurement {
            content_width: request.text.len() as f32 * request.font_size * 0.6,
            content_height: request.font_size * 1.2,
            line_height: request.font_size * 1.2,
            baseline: request.font_size * 0.8,
            ascent: request.font_size * 0.8,
            descent: request.font_size * 0.2,
            line_gap: 0.0,
            x_height: request.font_size * 0.5,
            cap_height: request.font_size * 0.7,
            advance_width: request.text.len() as f32 * request.font_size * 0.6,
            bounds: TextBounds::default(),
            line_measurements: vec![],
            total_character_count: request.text.len(),
            baseline_offset: 0.0,
            measured_at: std::time::Instant::now(),
        };

        if let Err(e) = self.cache.put(string_key, measurement.clone()) {
            eprintln!("Warning: Failed to cache measurement: {}", e);
        }

        Ok(measurement)
    }

    pub fn get_font_metrics(&self, _font_id: fontdb::ID, size: f32) -> Option<FontMetrics> {
        // Simplified font metrics - would integrate with font metrics cache
        Some(FontMetrics {
            units_per_em: 1000,
            ascent: (size * 0.8) as i16,
            descent: -(size * 0.2) as i16,
            line_gap: 0,
            x_height: Some((size * 0.5) as i16),
            cap_height: Some((size * 0.7) as i16),
            ideographic_baseline: Some(-(size * 0.1) as i16),
            hanging_baseline: Some((size * 0.8) as i16),
            mathematical_baseline: Some((size * 0.4) as i16),
            average_char_width: size * 0.6,
            max_char_width: size * 1.2,
            underline_position: size * -0.1,
            underline_thickness: size * 0.05,
            strikethrough_position: size * 0.4,
            strikethrough_thickness: size * 0.05,
        })
    }

    pub fn clear_cache(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache
            .clear()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

impl Default for EnhancedMeasurementCore {
    fn default() -> Self {
        Self::new().expect("Failed to create EnhancedMeasurementCore")
    }
}

/// Enhanced text measurer using goldylox
pub struct EnhancedTextMeasurer {
    core: EnhancedMeasurementCore,
    pub font_system: cosmyc_text::FontSystem,
    pub default_metrics: cosmyc_text::Metrics,
    pub default_shaping: cosmyc_text::Shaping,
    pub default_wrap: cosmyc_text::Wrap,
    pub default_align: cosmyc_text::Align,
    pub cache_manager: crate::analysis::caching::CacheManager,
}

impl EnhancedTextMeasurer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let core = EnhancedMeasurementCore::new()?;
        let font_system = cosmyc_text::FontSystem::new();
        let default_metrics = cosmyc_text::Metrics::new(16.0, 1.0);
        let default_shaping = cosmyc_text::Shaping::Advanced;
        let default_wrap = cosmyc_text::Wrap::Word;
        let default_align = cosmyc_text::Align::Left;
        let cache_manager = crate::analysis::caching::CacheManager;
        Ok(Self {
            core,
            font_system,
            default_metrics,
            default_shaping,
            default_wrap,
            default_align,
            cache_manager,
        })
    }

    pub fn measure_text(
        &mut self,
        request: &MeasurementRequest,
    ) -> Result<TextMeasurement, MeasurementError> {
        self.core.measure_text(request)
    }

    pub fn get_font_metrics(&self, font_id: fontdb::ID, size: f32) -> Option<FontMetrics> {
        self.core.get_font_metrics(font_id, size)
    }

    pub fn clear_cache(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.core.clear_cache()
    }

    /// Get measurement statistics
    pub fn get_stats(&self) -> crate::measurement::types::statistics::MeasurementStats {
        // Return default stats for now
        use crate::measurement::types::statistics::MeasurementStats;
        MeasurementStats {
            cache_hits: 0,
            cache_misses: 0,
            total_measurements: 0,
            font_metrics_cache_hits: 0,
            font_metrics_cache_misses: 0,
            baseline_cache_hits: 0,
            baseline_cache_misses: 0,
            evictions: 0,
            current_cache_size: 0,
            hit_rate: 0.0,
            font_metrics_hit_rate: 0.0,
            baseline_hit_rate: 0.0,
        }
    }
}

impl Default for EnhancedTextMeasurer {
    fn default() -> Self {
        Self::new().expect("Failed to create EnhancedTextMeasurer")
    }
}
