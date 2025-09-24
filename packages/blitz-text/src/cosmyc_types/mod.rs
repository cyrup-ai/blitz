//! Comprehensive cosmyc-text API integration and wrappers
//!
//! This module provides complete integration with all cosmyc-text APIs,
//! including advanced features like cursor management, text editing,
//! layout analysis, and font system integration.

// Re-export all core cosmyc-text types for convenience
pub use cosmyc_text::{
    // Database integration
    fontdb,
    Affinity,
    Align,
    Attrs,
    AttrsOwned,
    // Core types
    Buffer,
    CacheKey,
    CacheKeyFlags,
    CacheMetrics,
    // Color and styling
    Color,
    // Cursor and editing
    Cursor,
    // Text formatting
    Family,
    // Font types
    Font,
    FontFeatures,
    FontSystem,
    LayoutCursor,
    LayoutGlyph,
    LayoutLine,
    // Layout types
    LayoutRun,
    LetterSpacing,
    Metrics,
    Motion,
    PhysicalGlyph,
    Shaping,
    Stretch,
    Style,
    Weight,
    Wrap,
};

// Module declarations
pub mod buffer;
pub mod font_system;
pub mod glyph;
pub mod utilities;

// Re-export public types and utilities
pub use buffer::{
    EnhancedBuffer, LayoutRunInfo, BufferStateGuard, 
    CssWidthCalculationError, CssWidthMetrics, ThreadSafeBufferCalculator
};
pub use font_system::EnhancedFontSystem;
pub use glyph::GlyphInfo;
pub use utilities::{ColorUtils, CursorUtils, MetricsUtils};

// Tests extracted to tests/cosmyc_types_tests.rs for better performance
