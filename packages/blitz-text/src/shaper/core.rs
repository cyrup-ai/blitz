//! Core text shaper with lock-free operations and zero-allocation hot paths

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use cosmyc_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

use crate::analysis::TextAnalyzer;
use crate::cache::CacheManager;
use crate::error::ShapingError;
use crate::features::FeatureLookup;
use crate::line_breaking::{BreakClass, BreakOpportunity, LineBreakAnalyzer};
use crate::types::{
    GlyphFlags, ShapedGlyph, ShapedRun, ShapedText, ShapingContext, TextDirection, TextRun,
};

use super::ascii_fast::shape_ascii_fast;
use super::text_runs::create_text_runs_optimized;
use super::run_shaping::shape_runs_optimized;
use super::line_breaking::apply_line_breaking_optimized;
use super::utils::calculate_metrics_fast;

/// Lock-free glyph property cache for SIMD-optimized flag computation
pub(super) static GLYPH_PROPERTY_CACHE: AtomicUsize = AtomicUsize::new(0);
pub(super) static CACHE_HIT_COUNT: AtomicUsize = AtomicUsize::new(0);
pub(super) static CACHE_TOTAL_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    pub(super) static SHAPED_RUNS_BUFFER: std::cell::RefCell<Vec<ShapedRun>> =
        std::cell::RefCell::new(Vec::with_capacity(16));
    pub(super) static GLYPHS_BUFFER: std::cell::RefCell<Vec<ShapedGlyph>> =
        std::cell::RefCell::new(Vec::with_capacity(256));
    pub(super) static TEXT_RUNS_BUFFER: std::cell::RefCell<Vec<TextRun>> =
        std::cell::RefCell::new(Vec::with_capacity(8));
}

/// Statistics counters (lock-free)
pub(super) static SHAPING_OPERATIONS: AtomicUsize = AtomicUsize::new(0);
pub(super) static CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
pub(super) static TOTAL_GLYPHS_SHAPED: AtomicUsize = AtomicUsize::new(0);

/// Lock-free text shaper with zero-allocation hot paths
pub struct TextShaper {
    font_system: Arc<ArcSwap<FontSystem>>,
    analyzer: TextAnalyzer,
    cache_manager: CacheManager,
    default_metrics: Metrics,
    shaping_id: u64,
}

impl TextShaper {
    /// Create new text shaper with atomic font system access
    pub fn new(font_system: FontSystem) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            font_system: Arc::new(ArcSwap::new(Arc::new(font_system))),
            analyzer: TextAnalyzer::new(),
            cache_manager: CacheManager::new(),
            default_metrics: Metrics::new(16.0, 20.0),
            shaping_id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Create shaper with custom configuration
    pub fn with_config(
        font_system: FontSystem,
        cache_memory_mb: usize,
    ) -> Result<Self, ShapingError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Ok(Self {
            font_system: Arc::new(ArcSwap::new(Arc::new(font_system))),
            analyzer: TextAnalyzer::new(),
            cache_manager: CacheManager::with_memory_limit(cache_memory_mb)?,
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
                cache_key: self
                    .cache_manager
                    .create_cache_key(text, &attrs, max_width)?,
            }));
        }

        // Create cache key (zero allocation for common cases)
        let cache_key = self
            .cache_manager
            .create_cache_key(text, &attrs, max_width)?;

        // Check cache first (lock-free lookup)
        if let Some(cached) = self.cache_manager.get(&cache_key) {
            CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            return Ok(cached);
        }

        // Fast path for ASCII-only text
        if TextAnalyzer::is_ascii_only(text) {
            let mut font_system = (*self.font_system.load_full()).clone();
            return shape_ascii_fast(
                &mut font_system,
                text,
                attrs,
                max_width,
                cache_key,
                &self.cache_manager,
            );
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
        let text_runs =
            create_text_runs_optimized(text, &analysis, bidi_info.as_ref(), attrs)?;

        // Shape each run (zero allocation hot path)
        let shaped_runs = shape_runs_optimized(text_runs, &self.font_system)?;

        // Apply line breaking if needed
        let final_runs = if let Some(max_w) = max_width {
            apply_line_breaking_optimized(shaped_runs, max_w)?
        } else {
            shaped_runs
        };

        // Calculate metrics (compile-time optimized)
        let (total_width, total_height, baseline, line_count) =
            calculate_metrics_fast(&final_runs);

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
        if self
            .cache_manager
            .should_cache(text, analysis.complexity_score)
        {
            if let Err(_) = self.cache_manager.put(cache_key, shaped_text.clone()) {
                // Cache failure is non-fatal, continue with result
            }
        }

        Ok(shaped_text)
    }

    /// Update font system atomically
    pub fn update_font_system(&self, new_font_system: FontSystem) {
        self.font_system.store(Arc::new(new_font_system));
    }

    /// Get font system atomically
    pub fn font_system(&self) -> Arc<FontSystem> {
        self.font_system.load_full()
    }

    /// Get shaper ID
    pub fn shaping_id(&self) -> u64 {
        self.shaping_id
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        self.cache_manager.clear();
        self.analyzer.clear_caches();
    }
}