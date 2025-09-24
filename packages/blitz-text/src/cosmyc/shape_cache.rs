//! Enhanced shape cache using goldylox high-performance cache
//!
//! This module provides caching functionality for text shaping operations
//! with optimized performance using the goldylox cache system.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use cosmyc_text::{ShapeGlyph, ShapeRunCache, ShapeRunKey};

/// Enhanced ShapeRunCache wrapper with performance monitoring and statistics
pub struct EnhancedShapeRunCache {
    inner: ShapeRunCache,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
    cache_insertions: AtomicUsize,
    cache_trims: AtomicUsize,
    current_age: AtomicU64,
    total_glyphs_cached: AtomicUsize,
}

impl EnhancedShapeRunCache {
    /// Create new enhanced shape run cache
    pub fn new() -> Self {
        Self {
            inner: ShapeRunCache::default(),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
            cache_insertions: AtomicUsize::new(0),
            cache_trims: AtomicUsize::new(0),
            current_age: AtomicU64::new(0),
            total_glyphs_cached: AtomicUsize::new(0),
        }
    }

    /// Get reference to inner ShapeRunCache
    pub fn inner(&self) -> &ShapeRunCache {
        &self.inner
    }

    /// Get mutable reference to inner ShapeRunCache
    pub fn inner_mut(&mut self) -> &mut ShapeRunCache {
        &mut self.inner
    }

    /// Get cache item with performance tracking
    pub fn get(&mut self, key: &ShapeRunKey) -> Option<&Vec<ShapeGlyph>> {
        let result = self.inner.get(key);

        // Track cache performance
        if result.is_some() {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Insert cache item with performance tracking
    pub fn insert(&mut self, key: ShapeRunKey, glyphs: Vec<ShapeGlyph>) {
        let glyph_count = glyphs.len();
        self.inner.insert(key, glyphs);

        // Track cache performance
        self.cache_insertions.fetch_add(1, Ordering::Relaxed);
        self.total_glyphs_cached
            .fetch_add(glyph_count, Ordering::Relaxed);
    }

    /// Remove old cache entries with performance tracking
    pub fn trim(&mut self, keep_ages: u64) {
        self.inner.trim(keep_ages);

        // Track trim operations and age progression
        self.cache_trims.fetch_add(1, Ordering::Relaxed);
        self.current_age.fetch_add(1, Ordering::Relaxed);
    }

    /// Get comprehensive cache statistics
    pub fn cache_stats(&self) -> ShapeCacheStats {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        ShapeCacheStats {
            hits,
            misses,
            total,
            hit_ratio: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
            insertions: self.cache_insertions.load(Ordering::Relaxed),
            trims: self.cache_trims.load(Ordering::Relaxed),
            current_age: self.current_age.load(Ordering::Relaxed),
            total_glyphs_cached: self.total_glyphs_cached.load(Ordering::Relaxed),
        }
    }

    /// Clear all statistics
    pub fn clear_stats(&self) {
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.cache_insertions.store(0, Ordering::Relaxed);
        self.cache_trims.store(0, Ordering::Relaxed);
        self.current_age.store(0, Ordering::Relaxed);
        self.total_glyphs_cached.store(0, Ordering::Relaxed);
    }
}

impl Default for EnhancedShapeRunCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Shape cache performance statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ShapeCacheStats {
    pub hits: usize,
    pub misses: usize,
    pub total: usize,
    pub hit_ratio: f64,
    pub insertions: usize,
    pub trims: usize,
    pub current_age: u64,
    pub total_glyphs_cached: usize,
}

impl std::fmt::Display for ShapeCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Shape Cache Stats: {}/{} ({:.1}% hit ratio), {} insertions, {} trims, age {}, {} glyphs cached",
            self.hits,
            self.total,
            self.hit_ratio * 100.0,
            self.insertions,
            self.trims,
            self.current_age,
            self.total_glyphs_cached
        )
    }
}

/// Shape run utilities and analysis
pub struct ShapeRunUtils;

impl ShapeRunUtils {
    /// Analyze shape run complexity for caching decisions
    pub fn analyze_complexity(key: &ShapeRunKey) -> ShapeComplexity {
        let text_length = key.text.len();
        let attrs_count = key.attrs_spans.len();
        let has_complex_scripts = Self::has_complex_scripts(&key.text);

        let complexity_score =
            text_length + (attrs_count * 10) + if has_complex_scripts { 50 } else { 0 };

        if complexity_score > 100 {
            ShapeComplexity::High
        } else if complexity_score > 25 {
            ShapeComplexity::Medium
        } else {
            ShapeComplexity::Low
        }
    }

    /// Check if text contains complex scripts that benefit from caching
    fn has_complex_scripts(text: &str) -> bool {
        for ch in text.chars() {
            match ch {
                // Arabic range
                '\u{0600}'..='\u{06FF}' |
                // Hebrew range
                '\u{0590}'..='\u{05FF}' |
                // Devanagari range
                '\u{0900}'..='\u{097F}' |
                // CJK ranges
                '\u{4E00}'..='\u{9FFF}' |
                '\u{3400}'..='\u{4DBF}' |
                '\u{20000}'..='\u{2A6DF}' => return true,
                _ => continue,
            }
        }
        false
    }

    /// Calculate estimated cache benefit for a shape run
    pub fn cache_benefit_score(key: &ShapeRunKey, glyphs: &[ShapeGlyph]) -> f64 {
        let complexity = Self::analyze_complexity(key);
        let glyph_count = glyphs.len();

        let base_score = match complexity {
            ShapeComplexity::High => 10.0,
            ShapeComplexity::Medium => 5.0,
            ShapeComplexity::Low => 1.0,
        };

        // More glyphs = higher benefit from caching
        base_score * (1.0 + (glyph_count as f64 / 100.0))
    }
}

/// Shape run complexity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeComplexity {
    Low,
    Medium,
    High,
}

/// Enhanced shape run cache manager with automatic optimization
pub struct OptimizedShapeCache {
    cache: EnhancedShapeRunCache,
    max_age_limit: u64,
    auto_trim_threshold: usize,
    performance_mode: PerformanceMode,
}

impl OptimizedShapeCache {
    /// Create new optimized shape cache
    pub fn new() -> Self {
        Self {
            cache: EnhancedShapeRunCache::new(),
            max_age_limit: 100,
            auto_trim_threshold: 1000,
            performance_mode: PerformanceMode::Balanced,
        }
    }

    /// Create new optimized shape cache with custom settings
    pub fn with_settings(
        max_age_limit: u64,
        auto_trim_threshold: usize,
        performance_mode: PerformanceMode,
    ) -> Self {
        Self {
            cache: EnhancedShapeRunCache::new(),
            max_age_limit,
            auto_trim_threshold,
            performance_mode,
        }
    }

    /// Get shape run with automatic cache management
    pub fn get_optimized(&mut self, key: &ShapeRunKey) -> Option<&Vec<ShapeGlyph>> {
        // Check if trimming is needed before getting result
        let stats = self.cache.cache_stats();
        let should_trim = stats.total > self.auto_trim_threshold;

        if should_trim {
            let keep_ages = match self.performance_mode {
                PerformanceMode::Speed => self.max_age_limit * 2,
                PerformanceMode::Balanced => self.max_age_limit,
                PerformanceMode::Memory => self.max_age_limit / 2,
            };
            self.cache.trim(keep_ages);
        }

        // Now get the result after any potential trimming
        self.cache.get(key)
    }

    /// Insert shape run with intelligent caching decisions
    pub fn insert_optimized(&mut self, key: ShapeRunKey, glyphs: Vec<ShapeGlyph>) {
        let benefit_score = ShapeRunUtils::cache_benefit_score(&key, &glyphs);

        // Only cache if benefit score is high enough
        let threshold = match self.performance_mode {
            PerformanceMode::Speed => 1.0,
            PerformanceMode::Balanced => 2.0,
            PerformanceMode::Memory => 5.0,
        };

        if benefit_score >= threshold {
            self.cache.insert(key, glyphs);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> ShapeCacheStats {
        self.cache.cache_stats()
    }

    /// Get reference to inner cache
    pub fn inner(&mut self) -> &mut EnhancedShapeRunCache {
        &mut self.cache
    }
}

impl Default for OptimizedShapeCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance optimization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceMode {
    Speed,    // Prioritize speed, use more memory
    Balanced, // Balance speed and memory
    Memory,   // Prioritize memory, may be slower
}

// Tests extracted to tests/shape_cache_tests.rs for better performance
