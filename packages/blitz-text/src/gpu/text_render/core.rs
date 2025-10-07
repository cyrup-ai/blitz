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

use crate::custom_glyphs::{
    system::codepoint_to_compact_id, AtlasProcessor, CustomGlyphCache, CustomGlyphError,
    CustomGlyphRegistry,
};
use crate::gpu::GpuRenderConfig;

/// Enhanced TextRenderer with comprehensive performance monitoring and optimization
pub struct EnhancedTextRenderer {
    /// Inner glyphon TextRenderer
    pub(super) inner: TextRenderer,

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


    /// Create a new enhanced text renderer
    pub fn new(
        atlas: &mut TextAtlas,
        device: &Device,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let inner = TextRenderer::new(atlas, device, multisample, depth_stencil);

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
        let start_time = Instant::now();

        let result = self.inner.prepare(
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
        let start_time = Instant::now();

        let result = self.inner.render(atlas, viewport, pass);

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
        let mut custom_glyphs = Vec::new();

        // Iterate through all layout runs in the buffer
        for run in buffer.layout_runs() {
            // Iterate through glyphs in this run
            for glyph in run.glyphs.iter() {
                // Extract character(s) from glyph cluster
                if glyph.start < run.text.len() && glyph.end <= run.text.len() {
                    let char_range = &run.text[glyph.start..glyph.end];

                    // Get first character (emoji/icons are typically single chars)
                    if let Some(ch) = char_range.chars().next() {
                        let codepoint = ch as u32;

                        // Check if this is a custom glyph using existing detection
                        if AtlasProcessor::is_emoji_codepoint(codepoint)
                            || AtlasProcessor::is_icon_codepoint(codepoint)
                        {
                            // Map codepoint to compact ID using helper function
                            let Some(id) = codepoint_to_compact_id(codepoint) else {
                                continue;
                            };

                            // Create CustomGlyph for glyphon
                            let custom_glyph = CustomGlyph {
                                id,
                                left: glyph.x,
                                top: glyph.y,
                                width: glyph.w,
                                height: run.line_height,
                                color: glyph.color_opt.map(|c| Color::rgba(c.r(), c.g(), c.b(), c.a())),
                                snap_to_physical_pixel: true,
                                metadata: codepoint as usize,
                            };

                            custom_glyphs.push(custom_glyph);
                        }
                    }
                }
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
    pub fn inner(&self) -> &TextRenderer {
        &self.inner
    }

    /// Get mutable reference to inner TextRenderer for advanced usage
    pub fn inner_mut(&mut self) -> &mut TextRenderer {
        &mut self.inner
    }
}
