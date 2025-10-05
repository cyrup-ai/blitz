//! Advanced text shaping and layout engine for Blitz
//!
//! This crate provides comprehensive text processing capabilities including:
//! - Complex script shaping (Arabic, Devanagari, Thai, etc.)
//! - Bidirectional text support
//! - Advanced typography features (ligatures, kerning, OpenType features)
//! - High-performance caching and optimization
//! - Unicode-compliant text processing

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod analysis;
pub mod bidi;
pub mod cache;
pub mod cosmyc;
pub mod cosmyc_types;
pub mod custom_glyphs;
pub mod embedded_fallback;
pub mod error;
pub mod features;
pub mod gpu;
pub mod line_breaking;
pub mod measurement;
pub mod shaper;
pub mod shaping;
pub mod text_system;
pub mod types;

pub use bidi::{
    BidiError, BidiRenderOptions, BidiRenderTarget, BidiRenderer, CursorPosition, Direction,
    ProcessedBidi, SelectionRect, TextOrientation, UnicodeBidi, VisualRun, WritingMode,
};
pub use cosmyc::{
    editor::EditorStats, shape_cache::ShapeCacheStats, swash_cache::CacheStats,
    CosmicTextIntegration, EnhancedEditor, EnhancedShapeRunCache, EnhancedSwashCache,
    IntegrationMetrics, IntegrationOptimizationResult, IntegrationStats,
};
pub use cosmyc_types::{
    fontdb,
    Affinity,
    Align,
    Attrs,
    AttrsOwned,
    // Core cosmyc-text re-exports
    Buffer,
    CacheKey,
    CacheKeyFlags,
    CacheMetrics,
    Color,
    ColorUtils,
    Cursor,
    CursorUtils,
    // Enhanced wrappers
    EnhancedBuffer,
    EnhancedFontSystem,
    Family,
    Font,
    FontFeatures,
    FontSystem,
    GlyphInfo,
    LayoutCursor,
    LayoutGlyph,
    LayoutLine,
    LayoutRun,
    LayoutRunInfo,
    LetterSpacing,
    Metrics,
    MetricsUtils,
    Motion,
    PhysicalGlyph,
    Shaping,
    Stretch,
    Style,
    Weight,
    Wrap,
};
pub use custom_glyphs::{
    hash_color_key, AtlasCoords, CustomGlyph, CustomGlyphCache, CustomGlyphData, CustomGlyphError,
    CustomGlyphId, CustomGlyphRegistry, CustomGlyphSystem, GlyphKey, GlyphMetrics,
    GlyphSystemConfig, GlyphSystemStats,
};
pub use embedded_fallback::{
    ensure_embedded_fallback, load_embedded_fallback, EMBEDDED_FALLBACK_FAMILY,
};
pub use error::ShapingError;
pub use features::{CustomFeatures, FeatureLookup, FeatureSettings, FeaturesCache};
pub use gpu::{
    cache::GpuCacheStats, text_atlas::AtlasStats, viewport::ViewportStats, EnhancedGpuCache,
    EnhancedTextAtlas, EnhancedTextRenderer, EnhancedViewport, GpuRenderConfig, GpuRenderStats,
    GpuTextError, GpuTextResult,
};
pub use measurement::{
    extract_physical_glyphs, get_text_highlight_bounds, measure_layout_run_enhanced, BaselineInfo,
    CharacterPosition, EnhancedTextMeasurement, EnhancedTextMeasurer, FontMetrics, LineMeasurement,
    MeasurementStats, TextMeasurement, TextMeasurer,
};
pub use shaper::TextShaper;
pub use text_system::{
    Action,
    AttrsList,
    BufferLine,
    ComprehensiveStats,
    Edit,
    Editor,
    // Re-export FamilyOwned for font family handling
    FamilyOwned,
    LineEnding,
    PreparedText,
    RenderMetrics as SystemRenderMetrics,
    Selection,
    SystemOptimizationResult,
    SystemPerformanceStats,
    TextAreaConfig,
    TextSystemError,
    TextSystemResult,
    UnifiedTextConfig,
    UnifiedTextSystem,
};
pub use types::{
    FontKey, GlyphFlags, ScriptComplexity, ScriptRun, ShapedGlyph, ShapedRun, ShapedText,
    ShapingCacheKey, TextAnalysis, TextDirection, TextMetrics, TextRun,
};
