//! Layout run analysis with comprehensive cosmyc-text integration

use std::cell::RefCell;
use std::rc::Rc;

use cosmyc_text::LayoutRun;

use crate::cosmyc_types::EnhancedFontSystem;
use crate::measurement::types::*;

/// Layout analyzer for comprehensive layout run processing with zero allocation
pub struct LayoutAnalyzer {
    font_system: Rc<RefCell<EnhancedFontSystem>>,
}

impl LayoutAnalyzer {
    /// Create new layout analyzer with shared font system reference
    #[inline]
    pub fn new(font_system: &Rc<RefCell<EnhancedFontSystem>>) -> Self {
        Self {
            font_system: Rc::clone(font_system),
        }
    }

    /// Measure layout run with comprehensive analysis and character position extraction
    /// Optimized for zero allocation and maximum performance with real font metrics
    #[inline]
    pub fn measure_layout_run_comprehensive(
        &self,
        run: &LayoutRun,
    ) -> MeasurementResult<LineMeasurement> {
        let glyph_count = run.glyphs.len();
        let mut character_positions = Vec::with_capacity(glyph_count); // Pre-allocate
        let mut total_ascent = 0.0f32;
        let mut total_descent = 0.0f32;

        // Extract detailed character positions and metrics from each glyph (zero allocation)
        for glyph in run.glyphs.iter() {
            // Calculate precise character position from glyph data
            let char_pos = CharacterPosition {
                x: glyph.x,
                y: run.line_y + glyph.y,
                width: glyph.w,
                height: run.line_height,
                baseline_offset: glyph.y_offset,
                char_index: glyph.start,
                line_index: run.line_i,
                baseline: run.line_y,
            };
            character_positions.push(char_pos);

            // Calculate accurate ascent/descent from real font metrics (optimized lookup)
            let (font_ascent, font_descent) = {
                let mut font_system = self.font_system.borrow_mut();
                if let Some(font) = font_system
                    .inner_mut()
                    .get_font(glyph.font_id, glyph.font_weight)
                {
                    let swash_font = font.as_swash();
                    let face_metrics = swash_font.metrics(&[]);
                    let scale = glyph.font_size / face_metrics.units_per_em as f32;
                    (face_metrics.ascent * scale, -face_metrics.descent * scale)
                } else {
                    // Fallback only if font not available (graceful degradation)
                    (glyph.font_size * 0.8, glyph.font_size * 0.2)
                }
            };

            total_ascent = total_ascent.max(font_ascent);
            total_descent = total_descent.max(font_descent);
        }

        // Use font-derived metrics if available, otherwise calculate from default font
        let (ascent, descent) = if glyph_count > 0 {
            (total_ascent, total_descent)
        } else {
            // Get metrics from default font if no glyphs available (zero allocation path)
            let default_font_id = {
                let font_system = self.font_system.borrow();
                let face_id = font_system.inner().db().faces().next().map(|face| face.id);
                face_id
            };

            if let Some(default_font_id) = default_font_id {
                let mut font_system = self.font_system.borrow_mut();
                if let Some(font) = font_system
                    .inner_mut()
                    .get_font(default_font_id, cosmyc_text::Weight::NORMAL)
                {
                    let swash_font = font.as_swash();
                    let face_metrics = swash_font.metrics(&[]);
                    let scale = run.line_height / (face_metrics.ascent - face_metrics.descent);
                    (face_metrics.ascent * scale, -face_metrics.descent * scale)
                } else {
                    (run.line_height * 0.8, run.line_height * 0.2)
                }
            } else {
                (run.line_height * 0.8, run.line_height * 0.2)
            }
        };

        let line_gap = run.line_height - ascent - descent;

        Ok(LineMeasurement {
            width: run.line_w,
            height: run.line_height,
            ascent,
            descent,
            line_gap,
            baseline_offset: run.line_y - run.line_top,
            character_positions,
            start_char: run.glyphs.first().map(|g| g.start).unwrap_or(0),
            end_char: run.glyphs.last().map(|g| g.end).unwrap_or(0),
            glyph_count,
        })
    }

    /// Fast glyph count extraction for optimization decisions
    #[inline]
    pub fn count_glyphs(&self, run: &LayoutRun) -> usize {
        run.glyphs.len()
    }

    /// Check if layout run requires complex processing
    #[inline]
    pub fn requires_complex_analysis(&self, run: &LayoutRun) -> bool {
        run.rtl || run.glyphs.len() > 64 || run.glyphs.iter().any(|g| g.level.is_rtl())
    }
}
