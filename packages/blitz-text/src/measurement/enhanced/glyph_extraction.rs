//! Physical glyph extraction for rendering with zero allocation optimization

use cosmyc_text::{LayoutRun, PhysicalGlyph};

/// Glyph extractor for physical glyph rendering with optimized allocation patterns
pub struct GlyphExtractor;

impl GlyphExtractor {
    /// Create new glyph extractor (zero allocation)
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Extract physical glyphs for rendering with offset and scale transformations
    /// Optimized for iterator-based processing and rendering pipeline integration
    #[inline]
    pub fn extract_physical_glyphs(
        &self,
        run: &LayoutRun,
        offset: (f32, f32),
        scale: f32,
    ) -> Vec<PhysicalGlyph> {
        // Pre-allocate with exact capacity for zero reallocation
        let mut physical_glyphs = Vec::with_capacity(run.glyphs.len());

        // Process glyphs with iterator pattern for efficiency
        for glyph in run.glyphs.iter() {
            physical_glyphs.push(glyph.physical(offset, scale));
        }

        physical_glyphs
    }

    /// Extract physical glyphs with pre-allocated buffer (zero allocation variant)
    #[inline]
    pub fn extract_physical_glyphs_into(
        &self,
        run: &LayoutRun,
        offset: (f32, f32),
        scale: f32,
        buffer: &mut Vec<PhysicalGlyph>,
    ) {
        buffer.clear();
        buffer.reserve(run.glyphs.len());

        for glyph in run.glyphs.iter() {
            buffer.push(glyph.physical(offset, scale));
        }
    }

    /// Count glyphs that will be extracted (for pre-allocation decisions)
    #[inline]
    pub fn count_extractable_glyphs(&self, run: &LayoutRun) -> usize {
        run.glyphs.len()
    }

    /// Check if extraction is needed based on glyph content
    #[inline]
    pub fn extraction_needed(&self, run: &LayoutRun) -> bool {
        !run.glyphs.is_empty()
    }
}

impl Default for GlyphExtractor {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
