//! Glyph processing and layout analysis
//!
//! This module handles the processing of layout runs, glyph extraction,
//! and text bounds calculation for the text measurement system.

use cosmyc_text::{LayoutGlyph, LayoutRun};

use super::types::*;

/// Extract comprehensive measurements from LayoutRun using enhanced cosmyc-text APIs
pub fn measure_layout_run_enhanced(run: &LayoutRun) -> MeasurementResult<LineMeasurement> {
    let mut character_positions = Vec::new();
    let mut total_ascent = 0.0f32;
    let mut total_descent = 0.0f32;
    let mut glyph_count = 0;

    // Extract detailed character positions and metrics from each glyph
    for glyph in run.glyphs {
        glyph_count += 1;

        // Calculate character position from glyph data
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

        // Track ascent/descent for line metrics
        let glyph_ascent = (glyph.y_offset + glyph.font_size * 0.8).max(0.0);
        let glyph_descent = (glyph.font_size * 0.2 - glyph.y_offset).max(0.0);

        total_ascent = total_ascent.max(glyph_ascent);
        total_descent = total_descent.max(glyph_descent);
    }

    // Calculate average ascent/descent if we have glyphs
    let (ascent, descent) = if glyph_count > 0 {
        (total_ascent, total_descent)
    } else {
        // Fallback to line height proportions
        (run.line_height * 0.8, run.line_height * 0.2)
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

/// Extract physical glyphs for rendering using cosmyc-text PhysicalGlyph API
pub fn extract_physical_glyphs(
    glyphs: &[LayoutGlyph],
    offset: (f32, f32),
    scale: f32,
) -> Vec<cosmyc_text::PhysicalGlyph> {
    glyphs
        .iter()
        .map(|glyph| glyph.physical(offset, scale))
        .collect()
}

/// Get text highlight bounds using cosmyc-text LayoutRun highlight API
pub fn get_text_highlight_bounds(
    run: &LayoutRun,
    start_cursor: cosmyc_text::Cursor,
    end_cursor: cosmyc_text::Cursor,
) -> Option<(f32, f32)> {
    run.highlight(start_cursor, end_cursor)
}

/// Calculate text bounds from glyphs (ink bounds) and font metrics (logical bounds) - DEPRECATED
#[allow(dead_code)]
fn calculate_text_bounds(
    glyphs: &[LayoutGlyph],
    line_y: f32,
    line_height: f32,
    content_width: f32,
    content_height: f32,
) -> TextBounds {
    let mut ink_x_min = f32::MAX;
    let mut ink_y_min = f32::MAX;
    let mut ink_x_max = f32::MIN;
    let mut ink_y_max = f32::MIN;

    // Calculate ink bounds from actual glyph positions
    for glyph in glyphs {
        let glyph_x_min = glyph.x;
        let glyph_x_max = glyph.x + glyph.w;
        let glyph_y_min = line_y + glyph.y;
        let glyph_y_max = glyph_y_min + line_height;

        ink_x_min = ink_x_min.min(glyph_x_min);
        ink_x_max = ink_x_max.max(glyph_x_max);
        ink_y_min = ink_y_min.min(glyph_y_min);
        ink_y_max = ink_y_max.max(glyph_y_max);
    }

    // Handle empty glyph case
    if glyphs.is_empty() {
        ink_x_min = 0.0;
        ink_y_min = 0.0;
        ink_x_max = 0.0;
        ink_y_max = 0.0;
    }

    let ink_bounds = InkBounds {
        x_min: ink_x_min,
        y_min: ink_y_min,
        x_max: ink_x_max,
        y_max: ink_y_max,
    };

    let logical_bounds = LogicalBounds {
        x_min: 0.0,
        y_min: 0.0,
        x_max: content_width,
        y_max: content_height,
    };

    TextBounds {
        x_min: ink_x_min.min(0.0),
        y_min: ink_y_min.min(0.0),
        x_max: ink_x_max.max(content_width),
        y_max: ink_y_max.max(content_height),
        ink_bounds,
        logical_bounds,
    }
}

/// Calculate comprehensive text bounds using enhanced glyph analysis
pub fn calculate_text_bounds_enhanced(
    glyphs: &[LayoutGlyph],
    line_y: f32,
    line_height: f32,
    content_width: f32,
    content_height: f32,
) -> TextBounds {
    let mut ink_x_min = f32::MAX;
    let mut ink_y_min = f32::MAX;
    let mut ink_x_max = f32::MIN;
    let mut ink_y_max = f32::MIN;

    // Calculate ink bounds from actual glyph positions
    for glyph in glyphs {
        let glyph_x_min = glyph.x;
        let glyph_x_max = glyph.x + glyph.w;
        let glyph_y_min = line_y + glyph.y;
        let glyph_y_max = glyph_y_min + line_height;

        ink_x_min = ink_x_min.min(glyph_x_min);
        ink_x_max = ink_x_max.max(glyph_x_max);
        ink_y_min = ink_y_min.min(glyph_y_min);
        ink_y_max = ink_y_max.max(glyph_y_max);
    }

    // Handle empty glyph case
    if glyphs.is_empty() {
        ink_x_min = 0.0;
        ink_y_min = 0.0;
        ink_x_max = 0.0;
        ink_y_max = 0.0;
    }

    let ink_bounds = InkBounds {
        x_min: ink_x_min,
        y_min: ink_y_min,
        x_max: ink_x_max,
        y_max: ink_y_max,
    };

    // Logical bounds are based on font metrics and content area
    let logical_bounds = LogicalBounds {
        x_min: 0.0,
        y_min: 0.0,
        x_max: content_width,
        y_max: content_height,
    };

    TextBounds {
        x_min: ink_x_min.min(0.0),
        y_min: ink_y_min.min(0.0),
        x_max: ink_x_max.max(content_width),
        y_max: ink_y_max.max(content_height),
        ink_bounds,
        logical_bounds,
    }
}
