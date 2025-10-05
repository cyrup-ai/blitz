//! Enhanced measurement core using goldylox multi-tier caching

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use cosmyc_text::fontdb;
use goldylox::traits::CacheKey;
use goldylox::{Goldylox, GoldyloxBuilder};
use serde::{Deserialize, Serialize};

use crate::measurement::types::*;

// Statistics tracking (atomic counters for lock-free stats)
static CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
static CACHE_MISSES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_MEASUREMENTS: AtomicUsize = AtomicUsize::new(0);
static FONT_METRICS_CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
static FONT_METRICS_CACHE_MISSES: AtomicUsize = AtomicUsize::new(0);
static BASELINE_CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
static BASELINE_CACHE_MISSES: AtomicUsize = AtomicUsize::new(0);

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

/// Cache type for EnhancedMeasurementCore with fallback support
enum CacheType {
    Goldylox(Goldylox<String, TextMeasurement>),
    HashMap(Mutex<HashMap<String, TextMeasurement>>),
}

/// Enhanced measurement system using goldylox with HashMap fallback
pub struct EnhancedMeasurementCore {
    cache_type: CacheType,
}

impl EnhancedMeasurementCore {
    /// Convert EnhancedMeasurementKey to String for goldylox
    fn key_to_string(key: &EnhancedMeasurementKey) -> String {
        serde_json::to_string(key).unwrap_or_else(|_| format!("{:?}", key))
    }
}

impl EnhancedMeasurementCore {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use the global text measurement cache instead of creating a new one
        let cache = crate::cache::get_text_measurement_cache();
        
        println!("✅ EnhancedMeasurementCore using global Goldylox cache (singleton)");
        
        // Always use the global cache - no fallback needed since it's already initialized
        let cache_type = CacheType::Goldylox((*cache).clone());

        Ok(Self { cache_type })
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

        // Check cache based on cache type
        if let Some(cached) = self.get(&string_key) {
            CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            return Ok(cached);
        }

        // Cache miss - increment counter
        CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
        TOTAL_MEASUREMENTS.fetch_add(1, Ordering::Relaxed);

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
            measured_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        // Store in cache
        self.put(string_key, measurement.clone());

        Ok(measurement)
    }

    fn get(&self, key: &str) -> Option<TextMeasurement> {
        match &self.cache_type {
            CacheType::Goldylox(cache) => cache.get(&key.to_string()),
            CacheType::HashMap(cache) => {
                cache.lock().ok()?.get(key).cloned()
            }
        }
    }

    fn put(&self, key: String, value: TextMeasurement) {
        match &self.cache_type {
            CacheType::Goldylox(cache) => {
                if let Err(e) = cache.put(key, value) {
                    eprintln!("Warning: Failed to cache measurement in Goldylox: {}", e);
                }
            }
            CacheType::HashMap(cache) => {
                if let Ok(mut lock) = cache.lock() {
                    lock.insert(key, value);
                }
            }
        }
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
        match &self.cache_type {
            CacheType::Goldylox(cache) => {
                cache.clear().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            }
            CacheType::HashMap(cache) => {
                if let Ok(mut lock) = cache.lock() {
                    lock.clear();
                }
                Ok(())
            }
        }
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
        use crate::measurement::types::statistics::MeasurementStats;
        
        let cache_hits = CACHE_HITS.load(Ordering::Relaxed);
        let cache_misses = CACHE_MISSES.load(Ordering::Relaxed);
        let total = cache_hits + cache_misses;
        
        let font_metrics_hits = FONT_METRICS_CACHE_HITS.load(Ordering::Relaxed);
        let font_metrics_misses = FONT_METRICS_CACHE_MISSES.load(Ordering::Relaxed);
        let font_metrics_total = font_metrics_hits + font_metrics_misses;
        
        let baseline_hits = BASELINE_CACHE_HITS.load(Ordering::Relaxed);
        let baseline_misses = BASELINE_CACHE_MISSES.load(Ordering::Relaxed);
        let baseline_total = baseline_hits + baseline_misses;
        
        MeasurementStats {
            cache_hits: cache_hits as u64,
            cache_misses: cache_misses as u64,
            total_measurements: TOTAL_MEASUREMENTS.load(Ordering::Relaxed) as u64,
            font_metrics_cache_hits: font_metrics_hits as u64,
            font_metrics_cache_misses: font_metrics_misses as u64,
            baseline_cache_hits: baseline_hits as u64,
            baseline_cache_misses: baseline_misses as u64,
            evictions: 0, // Goldylox manages this internally
            current_cache_size: 0, // Can query from cache if needed
            hit_rate: if total > 0 { cache_hits as f32 / total as f32 } else { 0.0 },
            font_metrics_hit_rate: if font_metrics_total > 0 { font_metrics_hits as f32 / font_metrics_total as f32 } else { 0.0 },
            baseline_hit_rate: if baseline_total > 0 { baseline_hits as f32 / baseline_total as f32 } else { 0.0 },
        }
    }
}

impl Default for EnhancedTextMeasurer {
    fn default() -> Self {
        Self::new().expect("Failed to create EnhancedTextMeasurer")
    }
}
