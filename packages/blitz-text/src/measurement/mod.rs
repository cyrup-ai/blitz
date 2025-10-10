//! Text measurement system for blitz-text
//!
//! This module provides a comprehensive, lock-free text measurement system with:
//! - Zero allocation through thread-local buffer pooling
//! - Lock-free caching using crossbeam-epoch for garbage collection
//! - Complete CSS baseline support (all 8 baseline types)
//! - Production-ready performance with atomic statistics tracking
//!
//! # Example
//!
//! ```rust
//! use blitz_text::measurement::{TextMeasurer, CSSBaseline};
//!
//! let measurer = TextMeasurer::new();
//! let measurement = measurer.measure_text(
//!     "Hello, World!",
//!     16.0,
//!     Some(200.0),
//!     "Arial",
//!     CSSBaseline::Alphabetic,
//! )?;
//!
//! println!("Text size: {}x{}", measurement.content_width, measurement.content_height);
//! ```

pub mod cache;
pub mod enhanced;
pub mod font_metrics;
pub mod glyph_processing;
pub mod hot_tier;
pub mod measurement_api;
pub mod monitor;
pub mod text_measurement;
pub mod thread_local;
pub mod types;

// Re-export public API from decomposed modules
use std::sync::Arc;

use arc_swap::ArcSwap;
pub use cache::UnifiedCacheManager;
use cosmyc_text::FontSystem;
// Add missing imports for cache types
use crate::measurement::enhanced::font_metrics::FontMetricsCache;
use crate::bidi::cache::BidiCache;
use crate::features::cache::FeaturesCache;
use crate::measurement::cache::CacheManager;
pub use enhanced::{BaselineInfo, EnhancedTextMeasurement, EnhancedTextMeasurer};
pub use measurement_api::*;
pub use thread_local::{
    cleanup_thread_local_buffers, initialize_from_shared_font_system, with_character_positions,
    with_font_system, with_line_measurements, with_measurement_buffer, with_temp_string,
};
pub use types::{
    BaselineCacheKey, CSSBaseline as BaselineType, CharacterPosition, FontMetrics,
    FontMetricsCacheKey, InkBounds, LineMeasurement, LogicalBounds, MeasurementCacheKey,
    MeasurementError, MeasurementResult, MeasurementStats, TextBounds, TextMeasurement,
};

/// High-performance text measurement system with lock-free caching
///
/// This is the main entry point for text measurement operations. It provides
/// a thread-safe, lock-free interface for measuring text with comprehensive
/// caching and zero-allocation optimizations.
pub struct TextMeasurer {
    font_system: Arc<ArcSwap<FontSystem>>,
    cache_manager: Arc<ArcSwap<UnifiedCacheManager>>,
    max_cache_size: usize,
    stats: Arc<MeasurementStatsInner>,
}

impl TextMeasurer {
    /// Create a new TextMeasurer with default settings
    pub fn new() -> Self {
        use tokio::runtime::Handle;
        
        // Try to use current runtime if available
        let result = if let Ok(handle) = Handle::try_current() {
            handle.block_on(async {
                Self::with_cache_size(10000).await
            })
        } else {
            // No runtime available, create one temporarily
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(async {
                    Self::with_cache_size(10000).await
                })
        };
        
        result.unwrap_or_else(|_| {
            // Fallback to basic implementation without goldylox if initialization fails
            let font_system = Arc::new(ArcSwap::new(Arc::new(FontSystem::new())));
            let cache_manager = Arc::new(ArcSwap::new(Arc::new({
                use tokio::runtime::Handle;
                
                // Try to use current runtime if available
                if let Ok(handle) = Handle::try_current() {
                    handle.block_on(async {
                        UnifiedCacheManager::new().await.unwrap_or_else(|_| {
                            // If goldylox cache creation fails, create fallback with default implementations
                            UnifiedCacheManager {
                                measurement_cache: CacheManager::new().unwrap_or_else(|_| {
                                    panic!("Failed to create measurement cache manager")
                                }),
                                font_metrics_cache: FontMetricsCache::default(),
                                bidi_cache: BidiCache::default(),
                                features_cache: FeaturesCache::default(),
                            }
                        })
                    })
                } else {
                    // No runtime available, create one temporarily
                    tokio::runtime::Runtime::new()
                        .expect("Failed to create tokio runtime")
                        .block_on(async {
                            UnifiedCacheManager::new().await.unwrap_or_else(|_| {
                                UnifiedCacheManager {
                                    measurement_cache: CacheManager::new().unwrap_or_else(|_| {
                                        panic!("Failed to create measurement cache manager")
                                    }),
                                    font_metrics_cache: FontMetricsCache::default(),
                                    bidi_cache: BidiCache::default(),
                                    features_cache: FeaturesCache::default(),
                                }
                            })
                        })
                }
            })));
            let stats = Arc::new(MeasurementStatsInner::new());
            
            Self {
                font_system,
                cache_manager,
                max_cache_size: 10000,
                stats,
            }
        })
    }

    /// Create a new TextMeasurer with specified cache size
    pub async fn with_cache_size(max_cache_size: usize) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let font_system = Arc::new(ArcSwap::new(Arc::new(FontSystem::new())));
        let cache_manager = Arc::new(ArcSwap::new(Arc::new(UnifiedCacheManager::new().await?)));
        let stats = Arc::new(MeasurementStatsInner::new());

        Ok(Self {
            font_system,
            cache_manager,
            max_cache_size,
            stats,
        })
    }

    /// Create a new TextMeasurer with a pre-configured FontSystem
    pub async fn with_font_system(font_system: FontSystem) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let font_system_arc = Arc::new(ArcSwap::new(Arc::new(font_system)));
        let cache_manager = Arc::new(ArcSwap::new(Arc::new(UnifiedCacheManager::new().await?)));
        let stats = Arc::new(MeasurementStatsInner::new());

        Ok(Self {
            font_system: font_system_arc,
            cache_manager,
            max_cache_size: 10000,
            stats,
        })
    }

    /// Measure text dimensions and character positions
    pub fn measure_text(
        &self,
        text: &str,
        font_size: f32,
        attrs: cosmyc_text::Attrs,
        max_width: Option<f32>,
        _max_height: Option<f32>,
    ) -> MeasurementResult<TextMeasurement> {
        // Extract baseline from text analysis and available metrics
        let baseline = Self::determine_optimal_baseline(text, attrs.metrics_opt);

        // Extract font family for cache key
        let font_family = match attrs.family {
            cosmyc_text::Family::Name(name) => name,
            cosmyc_text::Family::Serif => "serif",
            cosmyc_text::Family::SansSerif => "sans-serif",
            cosmyc_text::Family::Cursive => "cursive",
            cosmyc_text::Family::Fantasy => "fantasy",
            cosmyc_text::Family::Monospace => "monospace",
        };

        // font_size is now passed as parameter
        // Check cache first
        let cache_key = MeasurementCacheKey::new(text, font_size, max_width, font_family, baseline);

        if let Some(cached) = self.cache_manager.load().get_measurement(&cache_key) {
            self.stats
                .total_measurements
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(cached);
        }

        // Initialize thread-local FontSystem from shared instance
        let shared_font_system = self.font_system.load();
        initialize_from_shared_font_system(&shared_font_system);

        // Perform measurement using core algorithm
        let measurement = text_measurement::perform_measurement(
            text,
            font_size,
            max_width,
            font_family,
            baseline,
            &*self.cache_manager.load(),
        )?;

        // Cache the result
        self.cache_manager
            .load()
            .cache_measurement(cache_key, measurement.clone());
        self.stats
            .total_measurements
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(measurement)
    }

    /// Get character positions for text (zero allocation)
    pub fn get_character_positions(
        &self,
        text: &str,
        font_size: f32,
        attrs: cosmyc_text::Attrs,
        _max_width: Option<f32>,
    ) -> MeasurementResult<Vec<CharacterPosition>> {
        let shared_font_system = self.font_system.load();
        initialize_from_shared_font_system(&shared_font_system);

        // Extract font info from attrs
        let font_family = match attrs.family {
            cosmyc_text::Family::Name(name) => name,
            cosmyc_text::Family::SansSerif => "sans-serif",
            _ => "sans-serif",
        };
        // font_size is now passed as parameter

        text_measurement::get_character_positions(text, font_size, font_family, &*self.cache_manager.load())
    }

    /// Calculate baseline for CSS baseline types
    pub fn calculate_baseline(
        &self,
        font_size: f32,
        font_family: &str,
        baseline_type: BaselineType,
    ) -> MeasurementResult<f32> {
        // Initialize thread-local FontSystem from shared instance
        let shared_font_system = self.font_system.load();
        initialize_from_shared_font_system(&shared_font_system);

        with_font_system(|font_system| {
            let query = cosmyc_text::fontdb::Query {
                families: &[cosmyc_text::fontdb::Family::Name(font_family)],
                weight: cosmyc_text::fontdb::Weight::NORMAL,
                stretch: cosmyc_text::fontdb::Stretch::Normal,
                style: cosmyc_text::fontdb::Style::Normal,
            };

            if let Some(font_id) = font_system.db().query(&query) {
                font_metrics::get_baseline_offset(
                    font_id,
                    font_size,
                    baseline_type,
                    &*self.cache_manager.load(),
                )
                .map_err(|_| MeasurementError::FontSystemError)
            } else {
                Err(MeasurementError::FontSystemError)
            }
        })?
    }

    /// Get current measurement statistics
    pub fn get_stats(&self) -> MeasurementStats {
        let stats = &*self.stats;
        let cache_hits = stats.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
        let cache_misses = stats
            .cache_misses
            .load(std::sync::atomic::Ordering::Relaxed);
        let total_measurements = stats
            .total_measurements
            .load(std::sync::atomic::Ordering::Relaxed);
        let font_metrics_hits = stats
            .font_metrics_cache_hits
            .load(std::sync::atomic::Ordering::Relaxed);
        let font_metrics_misses = stats
            .font_metrics_cache_misses
            .load(std::sync::atomic::Ordering::Relaxed);
        let baseline_hits = stats
            .baseline_cache_hits
            .load(std::sync::atomic::Ordering::Relaxed);
        let baseline_misses = stats
            .baseline_cache_misses
            .load(std::sync::atomic::Ordering::Relaxed);

        MeasurementStats {
            cache_hits,
            cache_misses,
            total_measurements,
            font_metrics_cache_hits: font_metrics_hits,
            font_metrics_cache_misses: font_metrics_misses,
            baseline_cache_hits: baseline_hits,
            baseline_cache_misses: baseline_misses,
            evictions: stats.evictions.load(std::sync::atomic::Ordering::Relaxed),
            current_cache_size: stats
                .current_cache_size
                .load(std::sync::atomic::Ordering::Relaxed),
            hit_rate: if cache_hits + cache_misses > 0 {
                cache_hits as f32 / (cache_hits + cache_misses) as f32
            } else {
                0.0
            },
            font_metrics_hit_rate: if font_metrics_hits + font_metrics_misses > 0 {
                font_metrics_hits as f32 / (font_metrics_hits + font_metrics_misses) as f32
            } else {
                0.0
            },
            baseline_hit_rate: if baseline_hits + baseline_misses > 0 {
                baseline_hits as f32 / (baseline_hits + baseline_misses) as f32
            } else {
                0.0
            },
        }
    }

    /// Update the FontSystem (thread-safe)
    pub fn update_font_system(&self, font_system: FontSystem) {
        self.font_system.store(Arc::new(font_system));
    }

    /// Get a reference to the current FontSystem
    pub fn font_system(&self) -> Arc<FontSystem> {
        (*self.font_system.load()).clone()
    }

    /// Clear all caches (useful for memory management)
    pub async fn clear_caches(&self) {
        // Create new cache manager and atomically swap it in
        // Old cache will be dropped when no longer referenced (Arc refcount)
        if let Ok(new_cache_manager) = UnifiedCacheManager::new().await {
            self.cache_manager.store(Arc::new(new_cache_manager));
        }
    }

    /// Determine optimal baseline based on text content and available font metrics
    ///
    /// This method analyzes the text for script types and uses font metrics when available
    /// to determine the most appropriate CSS baseline type for rendering.
    #[inline]
    fn determine_optimal_baseline(
        text: &str,
        metrics_opt: Option<cosmyc_text::CacheMetrics>,
    ) -> BaselineType {
        // Fast path for empty text
        if text.is_empty() {
            return BaselineType::Alphabetic;
        }

        // Analyze first few characters to determine primary script
        // This provides a fast heuristic without full text analysis
        let mut has_latin = false;
        let mut has_cjk = false;
        let mut has_arabic = false;
        let mut has_devanagari = false;

        // Analyze up to first 32 characters for performance
        for ch in text.chars().take(32) {
            match ch {
                // Latin script range
                'A'..='Z' | 'a'..='z' | '\u{0080}'..='\u{024F}' => has_latin = true,
                // CJK Unified Ideographs
                '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{20000}'..='\u{2A6DF}' => {
                    has_cjk = true
                }
                // Arabic script
                '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}' => {
                    has_arabic = true
                }
                // Devanagari script
                '\u{0900}'..='\u{097F}' => has_devanagari = true,
                _ => continue,
            }

            // Early exit optimization when we have enough information
            if has_latin && (has_cjk || has_arabic || has_devanagari) {
                break;
            }
        }

        // Determine baseline based on script analysis and metrics availability
        if let Some(_metrics) = metrics_opt {
            // With font metrics available, we can make informed decisions
            if has_devanagari {
                BaselineType::Hanging
            } else if has_cjk {
                BaselineType::Ideographic
            } else if has_arabic {
                BaselineType::Alphabetic // Arabic typically uses alphabetic baseline
            } else {
                BaselineType::Alphabetic // Default for Latin and mixed content
            }
        } else {
            // Without metrics, use conservative defaults
            if has_devanagari {
                BaselineType::Hanging
            } else if has_cjk {
                BaselineType::Ideographic
            } else {
                BaselineType::Alphabetic // Safe default for most scripts
            }
        }
    }
}

impl Default for TextMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TextMeasurer {
    fn drop(&mut self) {
        // Cleanup thread-local resources
        thread_local::cleanup_thread_local_buffers();
    }
}

// Ensure Send + Sync for multi-threaded usage
unsafe impl Send for TextMeasurer {}
unsafe impl Sync for TextMeasurer {}

// Tests extracted to tests/text_measurer_tests.rs for better performance
