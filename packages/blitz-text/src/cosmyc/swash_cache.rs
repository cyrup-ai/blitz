//! Legacy swash cache - now redirects to unified blitz-cache
//!
//! This module maintains compatibility while delegating to the unified cache system.

use std::sync::atomic::{AtomicUsize, Ordering};

use cosmyc_text::{CacheKey, FontSystem, SwashCache, SwashImage};

/// Enhanced SwashCache wrapper with performance monitoring and statistics
pub struct EnhancedSwashCache {
    inner: SwashCache,
    image_cache_hits: AtomicUsize,
    image_cache_misses: AtomicUsize,
    outline_cache_hits: AtomicUsize,
    outline_cache_misses: AtomicUsize,
    total_rasterizations: AtomicUsize,
}

impl EnhancedSwashCache {
    /// Create new enhanced swash cache
    pub fn new() -> Self {
        Self {
            inner: SwashCache::new(),
            image_cache_hits: AtomicUsize::new(0),
            image_cache_misses: AtomicUsize::new(0),
            outline_cache_hits: AtomicUsize::new(0),
            outline_cache_misses: AtomicUsize::new(0),
            total_rasterizations: AtomicUsize::new(0),
        }
    }

    /// Get reference to inner SwashCache
    pub fn inner(&self) -> &SwashCache {
        &self.inner
    }

    /// Get mutable reference to inner SwashCache
    pub fn inner_mut(&mut self) -> &mut SwashCache {
        &mut self.inner
    }

    /// Create a swash Image from a cache key with performance tracking
    pub fn get_image(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
    ) -> &Option<SwashImage> {
        let result = self.inner.get_image(font_system, cache_key);

        // Track cache performance
        if result.is_some() {
            self.image_cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.image_cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        self.total_rasterizations.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Create a swash Image from a cache key, without caching results
    pub fn get_image_uncached(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
    ) -> Option<SwashImage> {
        self.inner.get_image_uncached(font_system, cache_key)
    }

    /// Creates outline commands with performance tracking
    pub fn get_outline_commands(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
    ) -> Option<&[swash::zeno::Command]> {
        let result = self.inner.get_outline_commands(font_system, cache_key);

        // Track cache performance
        if result.is_some() {
            self.outline_cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.outline_cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Creates outline commands, without caching results
    pub fn get_outline_commands_uncached(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
    ) -> Option<Box<[swash::zeno::Command]>> {
        self.inner
            .get_outline_commands_uncached(font_system, cache_key)
    }

    /// Enumerate pixels in an Image with enhanced color handling
    pub fn with_pixels<F: FnMut(i32, i32, cosmyc_text::Color)>(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
        base: cosmyc_text::Color,
        f: F,
    ) {
        self.inner.with_pixels(font_system, cache_key, base, f);
    }

    /// Get image cache statistics
    pub fn image_cache_stats(&self) -> CacheStats {
        let hits = self.image_cache_hits.load(Ordering::Relaxed);
        let misses = self.image_cache_misses.load(Ordering::Relaxed);

        CacheStats {
            hits,
            misses,
            total: hits + misses,
            hit_ratio: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Get outline cache statistics
    pub fn outline_cache_stats(&self) -> CacheStats {
        let hits = self.outline_cache_hits.load(Ordering::Relaxed);
        let misses = self.outline_cache_misses.load(Ordering::Relaxed);

        CacheStats {
            hits,
            misses,
            total: hits + misses,
            hit_ratio: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Get total rasterization count
    pub fn total_rasterizations(&self) -> usize {
        self.total_rasterizations.load(Ordering::Relaxed)
    }

    /// Clear all statistics
    pub fn clear_stats(&self) {
        self.image_cache_hits.store(0, Ordering::Relaxed);
        self.image_cache_misses.store(0, Ordering::Relaxed);
        self.outline_cache_hits.store(0, Ordering::Relaxed);
        self.outline_cache_misses.store(0, Ordering::Relaxed);
        self.total_rasterizations.store(0, Ordering::Relaxed);
    }
}

impl Default for EnhancedSwashCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache performance statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub total: usize,
    pub hit_ratio: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache Stats: {}/{} ({:.1}% hit ratio)",
            self.hits,
            self.total,
            self.hit_ratio * 100.0
        )
    }
}

/// Enhanced rasterization utilities
pub struct RasterizationUtils;

impl RasterizationUtils {
    /// Convert SwashImage to RGBA8 buffer
    pub fn swash_image_to_rgba8(image: &SwashImage) -> Vec<u8> {
        let mut rgba_data = Vec::with_capacity(
            image.placement.width as usize * image.placement.height as usize * 4,
        );

        match image.content {
            swash::scale::image::Content::Mask => {
                // Convert grayscale mask to RGBA
                for &alpha in &image.data {
                    rgba_data.extend_from_slice(&[255, 255, 255, alpha]);
                }
            }
            swash::scale::image::Content::Color => {
                // Already in RGBA format
                rgba_data.extend_from_slice(&image.data);
            }
            swash::scale::image::Content::SubpixelMask => {
                // Convert subpixel mask to RGBA (simplified)
                for chunk in image.data.chunks(3) {
                    if chunk.len() == 3 {
                        rgba_data.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                    }
                }
            }
        }

        rgba_data
    }

    /// Get image dimensions
    pub fn image_dimensions(image: &SwashImage) -> (u32, u32) {
        (image.placement.width, image.placement.height)
    }

    /// Get image placement offset
    pub fn image_placement(image: &SwashImage) -> (i32, i32) {
        (image.placement.left, image.placement.top)
    }
}

// Tests extracted to tests/swash_cache_tests.rs
