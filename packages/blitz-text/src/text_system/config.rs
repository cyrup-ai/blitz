//! Configuration and error types for the unified text system
//!
//! This module contains all configuration structures, error types,
//! and result types used by the unified text system.

use crate::custom_glyphs::CustomGlyphError;
use crate::gpu::{GpuRenderConfig, GpuTextError};
use crate::measurement::MeasurementError;

/// Configuration for the unified text system
#[derive(Debug, Clone)]
pub struct UnifiedTextConfig {
    /// GPU rendering configuration
    pub gpu_config: Option<GpuRenderConfig>,
    /// Whether to enable automatic optimization
    pub auto_optimize: bool,
    /// Optimization frequency (every N operations)
    pub optimization_frequency: u32,
    /// Maximum memory usage before forcing optimization (in bytes)
    pub max_memory_usage: usize,
    /// Whether to enable performance monitoring
    pub enable_performance_monitoring: bool,
}

impl Default for UnifiedTextConfig {
    fn default() -> Self {
        Self {
            gpu_config: Some(GpuRenderConfig::default()),
            auto_optimize: true,
            optimization_frequency: 1000,
            max_memory_usage: 256 * 1024 * 1024, // 256MB
            enable_performance_monitoring: true,
        }
    }
}

/// Text area configuration
#[derive(Debug, Clone, Copy)]
pub struct TextAreaConfig {
    pub position: (f32, f32),
    pub scale: f32,
    pub bounds: glyphon::TextBounds,
    pub default_color: cosmyc_text::Color,
}

/// Prepared text ready for rendering
pub struct PreparedText {
    pub measurement: crate::measurement::TextMeasurement,
    pub buffer: cosmyc_text::Buffer,
    pub text_area_config: TextAreaConfig,
    pub preparation_time: std::time::Duration,
}

/// Enhanced render metrics - simplified to use goldylox metrics
#[derive(Debug, Clone)]
pub struct RenderMetrics {
    pub total_render_time: std::time::Duration,
    pub text_bounds: glyphon::TextBounds,
    pub glyph_count: usize,
}

/// Unified text system error types
#[derive(Debug, thiserror::Error)]
pub enum TextSystemError {
    #[error("Measurement error: {0}")]
    Measurement(#[from] MeasurementError),

    #[error("GPU error: {0}")]
    Gpu(#[from] GpuTextError),

    #[error("Custom glyph error: {0}")]
    CustomGlyph(#[from] CustomGlyphError),

    #[error("Font system error: {0}")]
    FontSystem(String),

    #[error("System configuration error: {0}")]
    Configuration(String),

    #[error("Resource allocation error: {0}")]
    ResourceAllocation(String),

    #[error("Prepare error: {0}")]
    Prepare(#[from] glyphon::PrepareError),

    #[error("Render error: {0}")]
    Render(#[from] glyphon::RenderError),
}

impl From<Box<dyn std::error::Error>> for TextSystemError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        TextSystemError::ResourceAllocation(err.to_string())
    }
}

/// Result type for unified text system operations
pub type TextSystemResult<T> = Result<T, TextSystemError>;
