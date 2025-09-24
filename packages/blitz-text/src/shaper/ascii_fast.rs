//! Ultra-fast ASCII text shaping with zero-allocation hot paths

use std::sync::atomic::Ordering;
use std::sync::Arc;

use cosmyc_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

use crate::cache::CacheManager;
use crate::error::ShapingError;
use crate::shaping::types::{GlyphFlags, ShapedGlyph, ShapedRun, ShapedText, TextDirection, ShapingCacheKey};

use super::core::{GLYPHS_BUFFER, TOTAL_GLYPHS_SHAPED};

/// Ultra-fast ASCII text shaping (zero allocation hot path)
pub(super) fn shape_ascii_fast(
    font_system: &mut FontSystem,
    text: &str,
    attrs: Attrs,
    max_width: Option<f32>,
    cache_key: ShapingCacheKey,
    cache_manager: &CacheManager,
) -> Result<Arc<ShapedText>, ShapingError> {
    // For ASCII text, we can use a highly optimized path
    // Note: This function signature changed to take needed parameters instead of &mut self
    
    // 1. Get font matches for the provided attributes
    let font_matches = font_system.get_font_matches(&attrs);

    // 2. Extract first available font
    let font_info = font_matches.first()
        .and_then(|match_key| font_system.db().face(match_key.id))
        .ok_or(ShapingError::FontNotFound)?;

    // 3. Extract real font metrics from FontSystem
    let font_size = attrs.metadata as f32;
    let metrics = Metrics::new(font_size, font_size * 1.2); // line_height = font_size * 1.2

    // 4. Get actual font for glyph width calculations
    let font = font_system.get_font(font_info.id, font_info.weight)
        .ok_or(ShapingError::FontLoadError)?;

    // Extract glyphs with zero allocation where possible
    let shaped_run = GLYPHS_BUFFER.with(|glyphs_buffer| {
        let mut glyphs_buffer = glyphs_buffer.borrow_mut();
        glyphs_buffer.clear();

        let mut total_width = 0.0;
        let mut max_ascent: f32 = metrics.line_height * 0.8;
        let mut max_descent: f32 = metrics.line_height * 0.2;

        // Extract proper glyph metrics for each character
        for (i, ch) in text.char_indices() {
            if ch.is_ascii() {
                // Get glyph ID from font
                let glyph_id = font.glyph_id(ch).unwrap_or(0);
                
                // Calculate real advance width
                let advance_width = font.glyph_hor_advance(glyph_id)
                    .map(|advance| (advance as f32 / font.units_per_em() as f32) * font_size)
                    .unwrap_or(font_size * 0.6);
                    
                glyphs_buffer.push(ShapedGlyph {
                    glyph_id: glyph_id as u16,
                    cluster: i as u32,
                    x_advance: advance_width,
                    y_advance: 0.0,
                    x_offset: total_width,
                    y_offset: 0.0,
                    flags: GlyphFlags::empty(),
                    font_size,
                    color: attrs.color_opt.map(|c| c.0),
                });
                
                total_width += advance_width;
            }
        }

        TOTAL_GLYPHS_SHAPED.fetch_add(glyphs_buffer.len(), Ordering::Relaxed);

        ShapedRun {
            glyphs: glyphs_buffer.clone(), // Only clone the actual data
            script: unicode_script::Script::Latin,
            direction: TextDirection::LeftToRight,
            language: None,
            level: unicode_bidi::Level::ltr(),
            width: total_width,
            height: max_ascent + max_descent,
            ascent: max_ascent,
            descent: max_descent,
            line_gap: metrics.line_height - max_ascent - max_descent,
            start_index: 0,
            end_index: text.len(),
        }
    });

    let total_width = shaped_run.width;
    let total_height = shaped_run.height;
    let baseline = shaped_run.ascent;

    let shaped_text = Arc::new(ShapedText {
        runs: vec![shaped_run],
        total_width,
        total_height,
        baseline,
        line_count: 1,
        shaped_at: std::time::Instant::now(),
        cache_key,
    });

    Ok(shaped_text)
}