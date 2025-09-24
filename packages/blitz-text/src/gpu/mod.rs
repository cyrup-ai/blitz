//! GPU-accelerated text rendering using glyphon integration
//!
//! This module provides complete GPU text rendering capabilities by integrating
//! with the glyphon crate, which offers WebGPU-based text rendering with texture
//! atlas management and efficient glyph caching.

pub mod cache;
pub mod text_atlas;
pub mod text_render;
pub mod viewport;

// Re-export enhanced GPU components
pub use cache::EnhancedGpuCache;
// Re-export cosmyc-text types needed for GPU rendering
pub use cosmyc_text::{
    Attrs, Buffer, CacheKey, Color, FontSystem, LayoutGlyph, LayoutRun, Metrics, PhysicalGlyph,
    SwashCache,
};
// Re-export core glyphon types for convenience
pub use glyphon::{
    Cache, ColorMode, ContentType, CustomGlyph, CustomGlyphId, PrepareError,
    RasterizeCustomGlyphRequest, RasterizedCustomGlyph, RenderError, Resolution, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};
pub use text_atlas::EnhancedTextAtlas;
pub use text_render::EnhancedTextRenderer;
pub use viewport::EnhancedViewport;

/// Performance statistics for GPU text rendering operations
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuRenderStats {
    /// Total number of render passes executed
    pub render_passes: u64,
    /// Total number of glyphs rendered across all passes
    pub total_glyphs_rendered: u64,
    /// Total number of text areas processed
    pub text_areas_processed: u64,
    /// Number of atlas growth events that occurred
    pub atlas_growth_events: u32,
    /// Number of vertex buffer reallocations
    pub vertex_buffer_reallocations: u32,
    /// Average glyphs per render pass
    pub avg_glyphs_per_pass: f64,
    /// Total GPU memory usage for atlases (estimated)
    pub estimated_atlas_memory_bytes: u64,
    /// Average render time in milliseconds
    pub average_render_time_ms: f64,
    /// Memory usage in megabytes
    pub memory_usage_mb: f64,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Average glyphs per rendering pass
    pub average_glyphs_per_pass: f64,
    /// Average preparation time in milliseconds
    pub average_preparation_time_ms: f64,
    /// Total preparation time in milliseconds
    pub total_preparation_time_ms: f64,
    /// Total render time in milliseconds
    pub total_render_time_ms: f64,
    /// Current vertex buffer size
    pub current_vertex_buffer_size: usize,
    /// Peak vertex buffer size
    pub peak_vertex_buffer_size: usize,
    /// System uptime in milliseconds
    pub uptime_ms: f64,
}

impl GpuRenderStats {
    /// Calculate the average glyphs per render pass
    pub fn calculate_avg_glyphs_per_pass(&mut self) {
        if self.render_passes > 0 {
            self.avg_glyphs_per_pass =
                self.total_glyphs_rendered as f64 / self.render_passes as f64;
        } else {
            self.avg_glyphs_per_pass = 0.0;
        }
    }

    /// Get memory usage per glyph (estimated)
    pub fn memory_per_glyph(&self) -> f64 {
        if self.total_glyphs_rendered > 0 {
            self.estimated_atlas_memory_bytes as f64 / self.total_glyphs_rendered as f64
        } else {
            0.0
        }
    }

    /// Get render efficiency score (0.0 to 1.0)
    pub fn efficiency_score(&self) -> f64 {
        if self.render_passes == 0 {
            return 1.0;
        }

        // Higher glyph count per pass = better efficiency
        let glyph_efficiency = (self.avg_glyphs_per_pass / 1000.0).min(1.0);

        // Fewer atlas growths = better efficiency
        let atlas_efficiency = if self.text_areas_processed > 0 {
            1.0 - (self.atlas_growth_events as f64 / self.text_areas_processed as f64).min(1.0)
        } else {
            1.0
        };

        // Fewer vertex buffer reallocations = better efficiency
        let buffer_efficiency = if self.render_passes > 0 {
            1.0 - (self.vertex_buffer_reallocations as f64 / self.render_passes as f64).min(1.0)
        } else {
            1.0
        };

        (glyph_efficiency + atlas_efficiency + buffer_efficiency) / 3.0
    }
}

/// Error types for GPU text rendering operations
#[derive(Debug, thiserror::Error)]
pub enum GpuTextError {
    #[error("GPU prepare error: {0}")]
    PrepareError(#[from] PrepareError),

    #[error("GPU render error: {0}")]
    RenderError(#[from] RenderError),

    #[error("Font system error: {0}")]
    FontSystemError(String),

    #[error("Atlas is full and cannot be grown further")]
    AtlasFull,

    #[error("Invalid text area configuration: {0}")]
    InvalidTextArea(String),

    #[error("GPU device lost or invalid")]
    DeviceError,

    #[error("Unsupported texture format: {format:?}")]
    UnsupportedFormat { format: wgpu::TextureFormat },

    #[error("Custom glyph error: {0}")]
    CustomGlyph(String),

    #[error("Prepare operation failed: {0}")]
    Prepare(String),
}

/// Result type for GPU text rendering operations
pub type GpuTextResult<T> = Result<T, GpuTextError>;

/// Configuration for GPU text rendering optimization
#[derive(Debug, Clone)]
pub struct GpuRenderConfig {
    /// Initial size for texture atlases
    pub initial_atlas_size: u32,
    /// Maximum size for texture atlases before failing
    pub max_atlas_size: u32,
    /// Initial vertex buffer size
    pub initial_vertex_buffer_size: u32,
    /// Whether to enable subpixel positioning
    pub enable_subpixel_positioning: bool,
    /// Color mode for text rendering
    pub color_mode: ColorMode,
    /// Whether to enable automatic atlas trimming
    pub enable_auto_trim: bool,
    /// Atlas trim frequency (every N render passes)
    pub atlas_trim_frequency: u32,
    /// Maximum render time in milliseconds
    pub max_render_time_ms: f64,
    /// Maximum vertex buffer reallocations before optimization
    pub max_vertex_buffer_reallocations: u32,
    /// Maximum memory usage in megabytes
    pub max_memory_usage_mb: f64,
    /// Minimum cache hit rate threshold
    pub min_cache_hit_rate: f64,
}

impl Default for GpuRenderConfig {
    fn default() -> Self {
        Self {
            initial_atlas_size: 512,
            max_atlas_size: 8192,
            initial_vertex_buffer_size: 4096,
            enable_subpixel_positioning: true,
            color_mode: ColorMode::Accurate,
            enable_auto_trim: true,
            atlas_trim_frequency: 60,  // Trim every 60 render passes
            max_render_time_ms: 16.67, // ~60 FPS
            max_vertex_buffer_reallocations: 10,
            max_memory_usage_mb: 512.0, // 512MB limit
            min_cache_hit_rate: 0.8,    // 80% minimum hit rate
        }
    }
}

impl GpuRenderConfig {
    /// Create a configuration optimized for speed over memory usage
    pub fn high_performance() -> Self {
        Self {
            initial_atlas_size: 1024,
            max_atlas_size: 16384,
            initial_vertex_buffer_size: 8192,
            enable_subpixel_positioning: true,
            color_mode: ColorMode::Accurate,
            enable_auto_trim: false, // Disable trimming for max speed
            atlas_trim_frequency: 0,
            max_render_time_ms: 16.0, // High performance target
            max_vertex_buffer_reallocations: 5,
            max_memory_usage_mb: 512.0,
            min_cache_hit_rate: 0.85,
        }
    }

    /// Create a configuration optimized for memory usage over speed
    pub fn low_memory() -> Self {
        Self {
            initial_atlas_size: 256,
            max_atlas_size: 2048,
            initial_vertex_buffer_size: 1024,
            enable_subpixel_positioning: false,
            color_mode: ColorMode::Web, // Use less memory
            enable_auto_trim: true,
            atlas_trim_frequency: 10, // Trim more frequently
            max_render_time_ms: 33.0, // More lenient for memory savings
            max_vertex_buffer_reallocations: 2,
            max_memory_usage_mb: 128.0,
            min_cache_hit_rate: 0.75,
        }
    }

    /// Create a balanced configuration for most use cases
    pub fn balanced() -> Self {
        Self::default()
    }
}
