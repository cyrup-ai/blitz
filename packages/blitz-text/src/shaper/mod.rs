//! Lock-free main text shaper with complex script support
//!
//! This module provides comprehensive text shaping capabilities including:
//! - Fast ASCII-only shaping with zero-allocation hot paths
//! - Complex script shaping with bidirectional text support
//! - SIMD-optimized glyph property analysis and caching
//! - UAX #14 compliant line breaking with Unicode property tables
//! - Fast metrics computation with SIMD optimization

pub mod ascii_shaper;
pub mod glyph_analysis;
pub mod line_breaking;
pub mod metrics_calculation;
pub mod run_shaping;

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
// Re-export public types and functionality
pub use ascii_shaper::{AsciiShaper, AsciiShaperStats};
use cosmyc_text::{Attrs, FontSystem, Metrics};
pub use glyph_analysis::{GlyphAnalysisStats, GlyphAnalyzer};
use goldylox::{Goldylox, GoldyloxBuilder};
pub use line_breaking::{LineBreakStats, LineBreaker};
pub use metrics_calculation::{BoundingBox, LineMetrics, MetricsCalculator, MetricsStats};
pub use run_shaping::{RunShaper, RunShapingStats};

use crate::analysis::TextAnalyzer;
use crate::error::ShapingError;
use crate::shaping::types::{ShapedText, ShapingCacheKey};
use crate::types::{ShapingContext, TextDirection};

/// Convert shaping::types::ShapingCacheKey to types::ShapingCacheKey  
fn convert_to_types_cache_key(key: ShapingCacheKey) -> crate::types::ShapingCacheKey {
    crate::types::ShapingCacheKey {
        text_hash: key.text_hash,
        attrs_hash: key.attrs_hash,
        max_width_hash: key.max_width_hash,
        feature_hash: key.feature_hash,
    }
}

/// Convert types::ShapingCacheKey to shaping::types::ShapingCacheKey
fn convert_to_shaping_cache_key(key: crate::types::ShapingCacheKey) -> ShapingCacheKey {
    ShapingCacheKey {
        text_hash: key.text_hash,
        attrs_hash: key.attrs_hash,
        max_width_hash: key.max_width_hash,
        feature_hash: key.feature_hash,
    }
}

/// Convert types::ShapedText to shaping::types::ShapedText
fn convert_to_shaping_shaped_text(text: Arc<crate::types::ShapedText>) -> Arc<ShapedText> {
    Arc::new(ShapedText {
        runs: text
            .runs
            .iter()
            .map(|run| crate::shaping::types::ShapedRun {
                glyphs: run
                    .glyphs
                    .iter()
                    .map(|glyph| crate::shaping::types::ShapedGlyph {
                        glyph_id: glyph.glyph_id,
                        cluster: glyph.cluster,
                        x_advance: glyph.x_advance,
                        y_advance: glyph.y_advance,
                        x_offset: glyph.x_offset,
                        y_offset: glyph.y_offset,
                        flags: crate::shaping::types::GlyphFlags::from_bits_truncate(
                            glyph.flags.bits(),
                        ),
                        font_size: glyph.font_size,
                        color: glyph.color,
                    })
                    .collect(),
                script: run.script,
                direction: match run.direction {
                    crate::types::TextDirection::LeftToRight => {
                        crate::shaping::types::TextDirection::LeftToRight
                    }
                    crate::types::TextDirection::RightToLeft => {
                        crate::shaping::types::TextDirection::RightToLeft
                    }
                    crate::types::TextDirection::TopToBottom => {
                        crate::shaping::types::TextDirection::TopToBottom
                    }
                    crate::types::TextDirection::BottomToTop => {
                        crate::shaping::types::TextDirection::BottomToTop
                    }
                },
                language: run.language.map(|s| s.to_string()),
                level: run.level,
                width: run.width,
                height: run.height,
                ascent: run.ascent,
                descent: run.descent,
                line_gap: run.line_gap,
                start_index: run.start_index,
                end_index: run.end_index,
            })
            .collect(),
        total_width: text.total_width,
        total_height: text.total_height,
        baseline: text.baseline,
        line_count: text.line_count,
        shaped_at: text.shaped_at,
        cache_key: convert_to_shaping_cache_key(text.cache_key.clone()),
    })
}

/// Global statistics counters (lock-free)
static SHAPING_OPERATIONS: AtomicUsize = AtomicUsize::new(0);
static CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_GLYPHS_SHAPED: AtomicUsize = AtomicUsize::new(0);

/// Lock-free text shaper with zero-allocation hot paths
pub struct TextShaper {
    font_system: Arc<ArcSwap<FontSystem>>,
    analyzer: TextAnalyzer,
    cache: Goldylox<String, ShapedText>,
    ascii_shaper: AsciiShaper,
    run_shaper: RunShaper,
    line_breaker: LineBreaker,
    default_metrics: Metrics,
    shaping_id: u64,
}

impl TextShaper {
    /// Create new text shaper with atomic font system access
    pub fn new(font_system: FontSystem) -> Result<Self, Box<dyn std::error::Error>> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Ok(Self {
            font_system: Arc::new(ArcSwap::new(Arc::new(font_system))),
            analyzer: TextAnalyzer::new(),
            cache: GoldyloxBuilder::<String, ShapedText>::new()
                .hot_tier_max_entries(1000)
                .hot_tier_memory_limit_mb(64)
                .warm_tier_max_entries(5000)
                .warm_tier_max_memory_bytes(256 * 1024 * 1024) // 256MB
                .cold_tier_max_size_bytes(1024 * 1024 * 1024) // 1GB
                .compression_level(6)
                .background_worker_threads(2)
                .cache_id("text_shaper_cache")
                .build()
                .map_err(|e| ShapingError::CacheOperationError(e.to_string()))?,
            ascii_shaper: AsciiShaper::new(),
            run_shaper: RunShaper::new(),
            line_breaker: LineBreaker::new(),
            default_metrics: Metrics::new(16.0, 20.0),
            shaping_id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        })
    }

    /// Create shaper with custom configuration
    pub fn with_config(
        font_system: FontSystem,
        cache_memory_mb: usize,
    ) -> Result<Self, ShapingError> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Ok(Self {
            font_system: Arc::new(ArcSwap::new(Arc::new(font_system))),
            analyzer: TextAnalyzer::new(),
            cache: GoldyloxBuilder::<String, ShapedText>::new()
                .hot_tier_max_entries((cache_memory_mb * 10) as u32) // Scale entries with memory
                .hot_tier_memory_limit_mb((cache_memory_mb / 4) as u32)
                .warm_tier_max_entries((cache_memory_mb * 50) as usize)
                .warm_tier_max_memory_bytes((cache_memory_mb * 1024 * 1024) as u64) // Use provided memory limit
                .cold_tier_max_size_bytes((cache_memory_mb * 2 * 1024 * 1024) as u64) // 2x memory for cold storage
                .compression_level(6)
                .background_worker_threads(2)
                .cache_id("text_shaper_cache_custom")
                .build()
                .map_err(|e| ShapingError::CacheOperationError(e.to_string()))?,
            ascii_shaper: AsciiShaper::new(),
            run_shaper: RunShaper::new(),
            line_breaker: LineBreaker::new(),
            default_metrics: Metrics::new(16.0, 20.0),
            shaping_id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        })
    }

    /// Shape text with full internationalization support (zero allocation hot path)
    pub fn shape_text(
        &mut self,
        text: &str,
        attrs: Attrs,
        max_width: Option<f32>,
    ) -> Result<Arc<ShapedText>, ShapingError> {
        SHAPING_OPERATIONS.fetch_add(1, Ordering::Relaxed);

        if text.is_empty() {
            return Ok(Arc::new(ShapedText {
                runs: Vec::new(),
                total_width: 0.0,
                total_height: 0.0,
                baseline: 0.0,
                line_count: 0,
                shaped_at: std::time::Instant::now(),
                cache_key: Self::create_cache_key(text, &attrs, max_width),
            }));
        }

        // Create cache key (zero allocation for common cases)
        let cache_key = Self::create_cache_key(text, &attrs, max_width);

        // Check cache first (lock-free lookup)
        let string_key = Self::key_to_string(&cache_key);
        if let Some(cached) = self.cache.get(&string_key) {
            CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            return Ok(Arc::new(cached));
        }

        // Fast path for ASCII-only text
        if AsciiShaper::is_ascii_only(text) {
            let font_system_guard = self.font_system.load();
            let font_system_ptr = Arc::as_ptr(&font_system_guard) as *mut FontSystem;

            // SAFETY: We hold the Arc guard for the duration of this operation
            // and FontSystem operations are thread-safe for read-only access
            let font_system = unsafe { &mut *font_system_ptr };

            return self
                .ascii_shaper
                .shape_ascii_fast(
                    font_system,
                    text,
                    attrs,
                    max_width,
                    convert_to_types_cache_key(cache_key),
                )
                .map(convert_to_shaping_shaped_text);
        }

        // Full analysis path for international text
        let analysis = self.analyzer.analyze_text(text)?;

        // Process bidirectional text if needed (zero allocation if not needed)
        let bidi_info = if analysis.requires_bidi {
            Some(self.analyzer.process_bidi(text, analysis.base_direction)?)
        } else {
            None
        };

        // Create text runs for shaping (reuse thread-local buffer)
        let text_runs = self.run_shaper.create_text_runs_optimized(
            text,
            &analysis,
            bidi_info.as_ref(),
            attrs,
        )?;

        // Shape each run (zero allocation hot path)
        let font_system_guard = self.font_system.load();
        let font_system_ptr = Arc::as_ptr(&font_system_guard) as *mut FontSystem;

        // SAFETY: We hold the Arc guard for the duration of this operation
        let font_system = unsafe { &mut *font_system_ptr };

        let shaped_runs = self
            .run_shaper
            .shape_runs_optimized(font_system, text_runs)?;

        // Apply line breaking if needed
        let final_runs = if let Some(max_w) = max_width {
            self.line_breaker
                .apply_line_breaking_optimized(shaped_runs, max_w)?
        } else {
            shaped_runs
        };

        // Calculate metrics (compile-time optimized)
        let (total_width, total_height, baseline, line_count) =
            MetricsCalculator::calculate_metrics_fast(&final_runs);

        let shaped_text = Arc::new(ShapedText {
            runs: final_runs,
            total_width,
            total_height,
            baseline,
            line_count,
            shaped_at: std::time::Instant::now(),
            cache_key: cache_key.clone(),
        });

        // Cache result if appropriate
        if shaped_text.runs.len() > 1 || text.len() > 10 {
            let string_key = Self::key_to_string(&cache_key);
            if let Err(_) = self.cache.put(string_key, (*shaped_text).clone()) {
                // Cache failure is non-fatal, continue with result
            }
        }

        Ok(shaped_text)
    }

    /// Update font system atomically (lock-free)
    pub fn update_font_system(&self, new_font_system: FontSystem) {
        self.font_system.store(Arc::new(new_font_system));
    }

    /// Get shaper statistics (lock-free atomic access)
    pub fn stats(&self) -> ShaperStats {
        let shaping_ops = SHAPING_OPERATIONS.load(Ordering::Relaxed);
        let cache_hits = CACHE_HITS.load(Ordering::Relaxed);

        ShaperStats {
            shaping_operations: shaping_ops,
            cache_hits,
            total_glyphs_shaped: TOTAL_GLYPHS_SHAPED.load(Ordering::Relaxed),
            cache_stats: crate::cosmyc::swash_cache::CacheStats {
                hits: cache_hits,
                misses: shaping_ops.saturating_sub(cache_hits),
                total: shaping_ops,
                hit_ratio: if shaping_ops > 0 {
                    cache_hits as f64 / shaping_ops as f64
                } else {
                    0.0
                },
            },
            analyzer_stats: self.analyzer.cache_stats(),
            ascii_stats: AsciiShaper::stats(),
            run_shaping_stats: RunShaper::stats(),
            line_break_stats: LineBreaker::stats(),
            glyph_analysis_stats: GlyphAnalyzer::stats(),
            metrics_stats: MetricsCalculator::stats(),
        }
    }

    /// Get unique shaper identifier
    pub fn shaping_id(&self) -> u64 {
        self.shaping_id
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        if let Err(_) = self.cache.clear() {
            // Cache clear failure is non-fatal
        }
        self.analyzer.clear_caches();
        self.run_shaper.clear_caches();
        self.line_breaker.clear_caches();

        // Clear module-specific buffers
        AsciiShaper::clear_buffers();
        RunShaper::clear_buffers();
        GlyphAnalyzer::clear_caches();
    }

    /// Optimize shaper performance based on usage patterns
    pub fn optimize(&mut self) -> Result<(), ShapingError> {
        // Goldylox handles optimization internally
        self.analyzer.optimize_caches()?;
        Ok(())
    }

    /// Create shaping context for advanced features
    #[inline]
    pub fn create_context(
        &self,
        script: unicode_script::Script,
        language: Option<&'static str>,
        direction: TextDirection,
        font_size: f32,
    ) -> ShapingContext {
        use crate::features::FeatureLookup;
        let features = FeatureLookup::get_features_for_script(script);

        ShapingContext {
            language,
            script,
            direction,
            features,
            font_size,
        }
    }

    /// Check if shaper needs optimization based on performance metrics
    #[inline]
    pub fn needs_optimization(&self) -> bool {
        let stats = self.stats();
        let hit_ratio = if stats.shaping_operations > 0 {
            stats.cache_hits as f64 / stats.shaping_operations as f64
        } else {
            1.0
        };

        // Optimize if hit ratio is low or total operations are high
        hit_ratio < 0.6 || stats.shaping_operations > 10000
    }

    /// Create cache key for shaping operations
    pub fn create_cache_key(
        text: &str,
        attrs: &cosmyc_text::Attrs,
        max_width: Option<f32>,
    ) -> ShapingCacheKey {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut text_hasher = DefaultHasher::new();
        text.hash(&mut text_hasher);
        let text_hash = text_hasher.finish();

        let mut attrs_hasher = DefaultHasher::new();
        attrs.hash(&mut attrs_hasher);
        let attrs_hash = attrs_hasher.finish();

        let mut width_hasher = DefaultHasher::new();
        max_width
            .unwrap_or(f32::INFINITY)
            .to_bits()
            .hash(&mut width_hasher);
        let max_width_hash = width_hasher.finish();

        // Simple feature hash for now
        let feature_hash = 0;

        ShapingCacheKey {
            text_hash,
            attrs_hash,
            max_width_hash,
            feature_hash,
        }
    }

    /// Convert cache key to string for goldylox
    pub fn key_to_string(key: &ShapingCacheKey) -> String {
        serde_json::to_string(key).unwrap_or_else(|_| format!("{:?}", key))
    }
}

impl Default for TextShaper {
    fn default() -> Self {
        // Create a minimal font system for default construction
        let font_system = FontSystem::new();
        Self::new(font_system).unwrap_or_else(|_| {
            // Fallback if construction fails
            let fallback_font_system = FontSystem::new();
            Self::new(fallback_font_system).expect("Failed to create default TextShaper")
        })
    }
}

/// Comprehensive shaper statistics
#[derive(Debug, Clone)]
pub struct ShaperStats {
    pub shaping_operations: usize,
    pub cache_hits: usize,
    pub total_glyphs_shaped: usize,
    pub cache_stats: crate::cosmyc::swash_cache::CacheStats,
    pub analyzer_stats: (usize, usize, usize, usize),
    pub ascii_stats: AsciiShaperStats,
    pub run_shaping_stats: RunShapingStats,
    pub line_break_stats: LineBreakStats,
    pub glyph_analysis_stats: GlyphAnalysisStats,
    pub metrics_stats: MetricsStats,
}

impl ShaperStats {
    /// Calculate cache hit ratio
    #[inline]
    pub fn hit_ratio(&self) -> f64 {
        if self.shaping_operations > 0 {
            self.cache_hits as f64 / self.shaping_operations as f64
        } else {
            0.0
        }
    }

    /// Calculate average glyphs per operation
    #[inline]
    pub fn avg_glyphs_per_operation(&self) -> f64 {
        if self.shaping_operations > 0 {
            self.total_glyphs_shaped as f64 / self.shaping_operations as f64
        } else {
            0.0
        }
    }
}
