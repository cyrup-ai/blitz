//! Enhanced text rendering module with comprehensive performance monitoring
//!
//! This module provides an enhanced text renderer that wraps glyphon's TextRenderer
//! with additional performance tracking, custom glyph support, and optimization features.

pub mod core;
pub mod preparation;
pub mod rendering;
pub mod statistics;
pub mod types;

// Re-export the main types for convenience
pub use core::EnhancedTextRenderer;

// Re-export cosmyc-text types
pub use cosmyc_text::{Buffer, Color, FontSystem, LayoutGlyph, LayoutRun, SwashCache};
// Re-export glyphon types for convenience
pub use glyphon::{
    CustomGlyph, PrepareError, RasterizeCustomGlyphRequest, RasterizedCustomGlyph, RenderError,
    Resolution, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
// Performance metrics are handled by goldylox cache system
