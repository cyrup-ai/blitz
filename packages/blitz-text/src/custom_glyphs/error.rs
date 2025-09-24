//! Error types for custom glyph operations

use thiserror::Error;

pub use glyphon::CustomGlyphId;

/// Comprehensive error handling for custom glyph operations
#[derive(Debug, Error)]
pub enum CustomGlyphError {
    #[error("Glyph registry is unavailable")]
    RegistryUnavailable,

    #[error("Custom glyph not found: ID {0}")]
    GlyphNotFound(CustomGlyphId),

    #[error("Atlas allocation failed for {width}x{height} glyph")]
    AtlasAllocationFailed { width: u16, height: u16 },

    #[error("Texture not available for glyph upload")]
    TextureNotAvailable,

    #[error("Glyph rasterization failed: {details}")]
    RasterizationFailed { details: String },

    #[error("Unicode processing error: {0}")]
    UnicodeError(String),

    #[error("Invalid text range for glyph")]
    InvalidTextRange,

    #[error("GPU resource creation failed: {0}")]
    GpuError(String),
}