//! Core text renderer implementation
//!
//! This module contains the main EnhancedTextRenderer struct and its basic operations.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize};
use std::time::Instant;

// Re-export cosmyc-text types
pub use cosmyc_text::{Buffer, Color, FontSystem, LayoutGlyph, LayoutRun, SwashCache};
// Re-export glyphon types for convenience
pub use glyphon::{
    CustomGlyph, PrepareError, RasterizeCustomGlyphRequest, RasterizedCustomGlyph, RenderError,
    Resolution, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::{DepthStencilState, Device, MultisampleState};

use crate::custom_glyphs::{CustomGlyphCache, CustomGlyphError, CustomGlyphRegistry};
use crate::gpu::GpuRenderConfig;

/// Enhanced TextRenderer with comprehensive performance monitoring and optimization
pub struct EnhancedTextRenderer {
    /// Inner glyphon TextRenderer (None in headless mode)
    pub(super) inner: Option<TextRenderer>,

    /// Performance statistics (atomic for thread safety)
    pub(super) render_passes: AtomicU64,
    pub(super) total_glyphs_rendered: AtomicU64,
    pub(super) text_areas_processed: AtomicU64,
    pub(super) vertex_buffer_reallocations: AtomicU32,
    pub(super) preparation_time_ns: AtomicU64,
    pub(super) render_time_ns: AtomicU64,

    /// Resource management
    pub(super) current_vertex_buffer_size: AtomicUsize,
    pub(super) peak_vertex_buffer_size: AtomicUsize,

    /// Custom glyph cache for emoji and symbols - lock-free with ArcSwap
    pub(super) custom_glyph_cache: arc_swap::ArcSwap<CustomGlyphCache>,

    /// Configuration
    pub(super) config: GpuRenderConfig,

    /// Performance tracking
    pub(super) last_trim_pass: AtomicU64,

    /// Statistics since last reset
    pub(super) stats_reset_time: Instant,
}

impl EnhancedTextRenderer {
    /// Create a headless text renderer for DOM operations without GPU context
    pub fn headless() -> Self {
        // No GPU context in headless mode
        let inner = None;

        // Create custom glyph registry and cache - lock-free initialization
        let registry = std::sync::Arc::new(CustomGlyphRegistry::new());
        let atlas_processor = std::sync::Arc::new(crate::custom_glyphs::atlas::AtlasProcessor);
        let custom_glyph_cache = arc_swap::ArcSwap::new(std::sync::Arc::new(
            CustomGlyphCache::new(registry, atlas_processor),
        ));

        Self {
            inner,
            render_passes: AtomicU64::new(0),
            total_glyphs_rendered: AtomicU64::new(0),
            text_areas_processed: AtomicU64::new(0),
            vertex_buffer_reallocations: AtomicU32::new(0),
            preparation_time_ns: AtomicU64::new(0),
            render_time_ns: AtomicU64::new(0),
            current_vertex_buffer_size: AtomicUsize::new(0),
            peak_vertex_buffer_size: AtomicUsize::new(0),
            custom_glyph_cache,
            config: GpuRenderConfig::default(),
            last_trim_pass: AtomicU64::new(0),
            stats_reset_time: Instant::now(),
        }
    }

    /// Create a new enhanced text renderer
    pub fn new(
        atlas: &mut TextAtlas,
        device: &Device,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let inner = Some(TextRenderer::new(atlas, device, multisample, depth_stencil));

        // Create custom glyph registry and cache - lock-free initialization
        let registry = std::sync::Arc::new(CustomGlyphRegistry::new());
        let atlas_processor = std::sync::Arc::new(crate::custom_glyphs::atlas::AtlasProcessor);
        let custom_glyph_cache = arc_swap::ArcSwap::new(std::sync::Arc::new(
            CustomGlyphCache::new(registry, atlas_processor),
        ));

        Self {
            inner,
            render_passes: AtomicU64::new(0),
            total_glyphs_rendered: AtomicU64::new(0),
            text_areas_processed: AtomicU64::new(0),
            vertex_buffer_reallocations: AtomicU32::new(0),
            preparation_time_ns: AtomicU64::new(0),
            render_time_ns: AtomicU64::new(0),
            current_vertex_buffer_size: AtomicUsize::new(0),
            peak_vertex_buffer_size: AtomicUsize::new(0),
            custom_glyph_cache,
            config: GpuRenderConfig::default(),
            last_trim_pass: AtomicU64::new(0),
            stats_reset_time: Instant::now(),
        }
    }

    /// Create a new enhanced text renderer with custom configuration
    pub fn with_config(
        atlas: &mut TextAtlas,
        device: &Device,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
        config: GpuRenderConfig,
    ) -> Self {
        let mut renderer = Self::new(atlas, device, multisample, depth_stencil);
        renderer.config = config;
        renderer
    }

    /// Enhanced preparation method for text rendering
    pub fn prepare_enhanced<'a>(
        &mut self,
        device: &Device,
        queue: &wgpu::Queue,
        font_system: &mut FontSystem,
        atlas: &mut TextAtlas,
        viewport: &Viewport,
        text_areas: impl IntoIterator<Item = TextArea<'a>>,
        swash_cache: &mut cosmyc_text::SwashCache,
    ) -> Result<(), PrepareError> {
        let Some(ref mut inner) = self.inner else {
            // Headless mode: no-op but track the call
            return Ok(());
        };

        let start_time = Instant::now();

        let result = inner.prepare(
            device,
            queue,
            font_system,
            atlas,
            viewport,
            text_areas,
            swash_cache,
        );

        // Update performance metrics
        let prep_time = start_time.elapsed().as_nanos() as u64;
        self.preparation_time_ns
            .fetch_add(prep_time, std::sync::atomic::Ordering::Relaxed);

        result
    }

    /// Enhanced rendering method with performance tracking
    pub fn render_enhanced<'a>(
        &self,
        atlas: &TextAtlas,
        viewport: &Viewport,
        pass: &mut wgpu::RenderPass<'a>,
    ) -> Result<(), RenderError> {
        let Some(ref inner) = self.inner else {
            // Headless mode: no-op but track the call
            return Ok(());
        };

        let start_time = Instant::now();

        let result = inner.render(atlas, viewport, pass);

        // Update performance metrics
        let render_time = start_time.elapsed().as_nanos() as u64;
        self.render_time_ns
            .fetch_add(render_time, std::sync::atomic::Ordering::Relaxed);
        self.render_passes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        result
    }

    /// Get custom glyphs for a cosmyc-text buffer (emoji, symbols, private use)
    pub(super) fn get_glyphs_for_buffer(
        &self,
        buffer: &Buffer,
    ) -> Result<Vec<CustomGlyph>, CustomGlyphError> {
        // Lock-free atomic cache access with blazing-fast performance
        let _cache = self.custom_glyph_cache.load();

        // Extract custom glyphs from buffer layout runs
        let custom_glyphs = Vec::new();

        for run in buffer.layout_runs() {
            for _glyph in run.glyphs.iter() {
                // Check if this is a custom glyph (e.g., emoji, symbols)
                // For now, return empty vec as placeholder until proper glyph detection is implemented
                // TODO: Implement proper custom glyph detection from buffer layout
            }
        }

        Ok(custom_glyphs)
    }

    /// Get the current configuration
    pub fn config(&self) -> &GpuRenderConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: GpuRenderConfig) {
        self.config = config;
    }

    /// Get reference to inner TextRenderer for advanced usage
    pub fn inner(&self) -> Option<&TextRenderer> {
        self.inner.as_ref()
    }

    /// Get mutable reference to inner TextRenderer for advanced usage
    pub fn inner_mut(&mut self) -> Option<&mut TextRenderer> {
        self.inner.as_mut()
    }
}
