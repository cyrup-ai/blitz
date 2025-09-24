//! A [`vello`] backend for the [`anyrender`] 2D drawing abstraction
mod debug;
mod error;
mod image_renderer;
mod scene;
mod window_renderer;

pub mod custom_paint_source;
pub mod wgpu_context;

use std::num::NonZeroUsize;

pub use custom_paint_source::*;
use debug::DebugTimer;
pub use error::{TextRenderError, TextRenderResult};
pub use image_renderer::VelloImageRenderer;
pub use scene::VelloScenePainter;
pub use wgpu;
pub use window_renderer::VelloWindowRenderer;

#[cfg(target_os = "macos")]
const DEFAULT_THREADS: Option<NonZeroUsize> = NonZeroUsize::new(1);
#[cfg(not(target_os = "macos"))]
const DEFAULT_THREADS: Option<NonZeroUsize> = None;

use std::cell::RefCell;
use std::rc::Rc;

/// State management for glyphon text rendering
pub struct GlyphonState {
    /// The main text renderer that draws to GPU
    pub text_renderer: glyphon::TextRenderer,
    /// GPU texture atlas for caching glyphs
    pub text_atlas: glyphon::TextAtlas,
    /// Shared font system between blitz-text and glyphon  
    pub font_system: Rc<RefCell<blitz_text::FontSystem>>,
    /// Cache for font rasterization
    pub swash_cache: glyphon::SwashCache,
    /// Viewport configuration for the window
    pub viewport: glyphon::Viewport,
    /// Shared cache for pipelines and resources
    pub cache: glyphon::Cache,
    /// Text areas collected during scene building
    pub pending_text_areas: Vec<PendingTextArea>,
}

impl GlyphonState {
    /// Create a new GlyphonState with all required components
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        // Create font system - expensive operation done once
        let font_system = Rc::new(RefCell::new(blitz_text::FontSystem::new()));

        // Create swash cache for glyph rasterization
        let swash_cache = glyphon::SwashCache::new();

        // Create cache for shared resources (pipelines, shaders, etc.)
        let cache = glyphon::Cache::new(device);

        // Create text atlas for GPU glyph caching - 4096x4096 texture
        let mut text_atlas = glyphon::TextAtlas::new(device, queue, &cache, format);

        // Create text renderer with multisample and depth stencil configuration
        let text_renderer = glyphon::TextRenderer::new(
            &mut text_atlas,
            device,
            wgpu::MultisampleState::default(),
            None, // No depth stencil for now
        );

        // Create viewport with proper constructor
        let mut viewport = glyphon::Viewport::new(device, &cache);

        // Update viewport with window dimensions
        viewport.update(queue, glyphon::Resolution { width, height });

        Self {
            text_renderer,
            text_atlas,
            font_system,
            swash_cache,
            viewport,
            cache,
            pending_text_areas: Vec::new(),
        }
    }

    /// Update viewport dimensions when window is resized
    pub fn resize(&mut self, width: u32, height: u32, queue: &wgpu::Queue) {
        self.viewport
            .update(queue, glyphon::Resolution { width, height });
    }

    /// Add a text area to be rendered in the next frame
    pub fn add_text_area(&mut self, text_area: PendingTextArea) {
        self.pending_text_areas.push(text_area);
    }

    /// Clear all pending text areas (called after rendering)
    pub fn clear_pending(&mut self) {
        self.pending_text_areas.clear();
    }
}

/// A text area waiting to be rendered
pub struct PendingTextArea {
    /// The blitz-text buffer containing shaped text
    pub buffer: Rc<blitz_text::Buffer>,
    /// X position in screen coordinates
    pub left: f32,
    /// Y position in screen coordinates  
    pub top: f32,
    /// Scale factor for high DPI displays
    pub scale: f32,
    /// Text color
    pub color: glyphon::Color,
    /// Clipping bounds for the text
    pub bounds: glyphon::TextBounds,
    /// Z-order for correct layering
    pub z_index: f32,
}
