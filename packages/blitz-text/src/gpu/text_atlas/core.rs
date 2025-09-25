//! Core text atlas implementation
//!
//! This module contains the main EnhancedTextAtlas struct and its basic operations.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

// Re-export cosmyc-text types
pub use cosmyc_text::{FontSystem, SwashCache};
// Re-export glyphon types for convenience
pub use glyphon::{
    Cache, ColorMode, ContentType, RasterizeCustomGlyphRequest, RasterizedCustomGlyph, TextAtlas,
};
use wgpu::{Device, Queue, TextureFormat};

use super::types::{AtlasGrowthEvent, TrimEvent};
use crate::gpu::GpuRenderConfig;

/// Enhanced TextAtlas with comprehensive performance monitoring and optimization
pub struct EnhancedTextAtlas {
    /// Inner glyphon TextAtlas
    pub(super) inner: TextAtlas,

    /// Performance statistics (atomic for thread safety)
    pub(super) cache_hits: AtomicU64,
    pub(super) cache_misses: AtomicU64,
    pub(super) atlas_growths: AtomicU32,
    pub(super) trim_operations: AtomicU64,
    pub(super) glyph_allocations: AtomicU64,
    pub(super) glyph_deallocations: AtomicU64,

    /// Memory tracking
    pub(super) estimated_memory_usage: AtomicUsize,
    pub(super) peak_memory_usage: AtomicUsize,
    pub(super) color_atlas_size: AtomicU32,
    pub(super) mask_atlas_size: AtomicU32,

    /// Atlas growth tracking
    pub(super) growth_events: parking_lot::Mutex<Vec<AtlasGrowthEvent>>,

    /// Configuration
    pub(super) config: GpuRenderConfig,

    /// Performance tracking
    pub(super) last_optimization_time: parking_lot::Mutex<Instant>,
    pub(super) stats_reset_time: Instant,
}

impl EnhancedTextAtlas {
    /// Create a headless text atlas for DOM operations without GPU context
    pub fn headless() -> Self {
        // Use unsafe mem::zeroed for placeholder - will be replaced when GPU context available
        let inner = unsafe { std::mem::zeroed() };

        Self {
            inner,
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            atlas_growths: AtomicU32::new(0),
            trim_operations: AtomicU64::new(0),
            glyph_allocations: AtomicU64::new(0),
            glyph_deallocations: AtomicU64::new(0),
            estimated_memory_usage: AtomicUsize::new(0),
            peak_memory_usage: AtomicUsize::new(0),
            color_atlas_size: AtomicU32::new(0),
            mask_atlas_size: AtomicU32::new(0),
            growth_events: parking_lot::Mutex::new(Vec::new()),
            config: GpuRenderConfig::default(),
            last_optimization_time: parking_lot::Mutex::new(Instant::now()),
            stats_reset_time: Instant::now(),
        }
    }

    /// Create a new enhanced text atlas
    pub fn new(device: &Device, queue: &Queue, cache: &Cache, format: TextureFormat) -> Self {
        let inner = TextAtlas::new(device, queue, cache, format);

        Self {
            inner,
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            atlas_growths: AtomicU32::new(0),
            trim_operations: AtomicU64::new(0),
            glyph_allocations: AtomicU64::new(0),
            glyph_deallocations: AtomicU64::new(0),
            estimated_memory_usage: AtomicUsize::new(0),
            peak_memory_usage: AtomicUsize::new(0),
            color_atlas_size: AtomicU32::new(256), // Default initial size
            mask_atlas_size: AtomicU32::new(256),
            growth_events: parking_lot::Mutex::new(Vec::new()),
            config: GpuRenderConfig::default(),
            last_optimization_time: parking_lot::Mutex::new(Instant::now()),
            stats_reset_time: Instant::now(),
        }
    }

    /// Create a new enhanced text atlas with specific color mode
    pub fn with_color_mode(
        device: &Device,
        queue: &Queue,
        cache: &Cache,
        format: TextureFormat,
        color_mode: ColorMode,
    ) -> Self {
        let inner = TextAtlas::with_color_mode(device, queue, cache, format, color_mode);

        let mut atlas = Self::new(device, queue, cache, format);
        atlas.inner = inner;
        atlas
    }

    /// Create a new enhanced text atlas with custom configuration
    pub fn with_config(
        device: &Device,
        queue: &Queue,
        cache: &Cache,
        format: TextureFormat,
        config: GpuRenderConfig,
    ) -> Self {
        let mut atlas = Self::new(device, queue, cache, format);
        atlas.config = config;
        atlas
    }

    /// Trim unused glyphs with enhanced monitoring
    pub fn trim_enhanced(&mut self) {
        let start_time = Instant::now();

        // Get memory usage before trimming
        let memory_before = self.estimated_memory_usage.load(Ordering::Relaxed);

        // Call inner trim method
        self.inner.trim();

        // Update statistics
        self.trim_operations.fetch_add(1, Ordering::Relaxed);

        // Estimate memory saved (simplified calculation)
        let memory_after = self.estimated_memory_usage.load(Ordering::Relaxed);
        let memory_saved = memory_before.saturating_sub(memory_after);

        // Track trim operation
        let trim_event = TrimEvent {
            timestamp: start_time,
            memory_saved,
            duration: start_time.elapsed(),
        };

        // Log significant memory savings
        if memory_saved > 1024 * 1024 {
            // > 1MB saved
            log::debug!(
                "Atlas trim saved {} bytes in {:?}",
                memory_saved,
                trim_event.duration
            );
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &GpuRenderConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: GpuRenderConfig) {
        self.config = config;
    }

    /// Get reference to inner TextAtlas for advanced usage
    pub fn inner(&self) -> &TextAtlas {
        &self.inner
    }

    /// Get mutable reference to inner TextAtlas for advanced usage
    pub fn inner_mut(&mut self) -> &mut TextAtlas {
        &mut self.inner
    }
}
