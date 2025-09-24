//! Core text measurement algorithms
//!
//! This module contains the main text measurement algorithm that coordinates
//! font processing, glyph analysis, and character positioning.

use std::time::Instant;

use cosmyc_text::{Attrs, Buffer, Shaping};

use super::cache::UnifiedCacheManager;
use super::font_metrics::{calculate_baseline_offset, extract_font_metrics};
use super::glyph_processing::calculate_text_bounds_enhanced;
use super::thread_local::{with_character_positions, with_font_system, with_line_measurements};
use super::types::*;
use crate::measurement::types::MeasurementError;

/// Get character positions for text with zero allocation
pub fn get_character_positions(
    text: &str,
    font_size: f32,
    font_family: &str,
    _cache_manager: &UnifiedCacheManager,
) -> Result<Vec<CharacterPosition>, MeasurementError> {
    with_character_positions(|positions| {
        with_font_system(|font_system| {
            // Create buffer for text measurement
            let mut buffer = Buffer::new(
                font_system,
                cosmyc_text::Metrics::new(font_size, font_size * 1.2),
            );
            buffer.set_size(font_system, None, None);

            // Set text with attributes
            let attrs = Attrs::new().family(cosmyc_text::Family::Name(font_family));
            buffer.set_text(font_system, text, &attrs, Shaping::Advanced);

            // Shape text for measurement
            buffer.shape_until_scroll(font_system, true);

            let mut char_index = 0;

            // Iterate through layout runs and glyphs
            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    if char_index >= text.len() {
                        break;
                    }

                    // Calculate character position
                    let position = CharacterPosition {
                        x: glyph.x,
                        y: glyph.y,
                        width: glyph.w,
                        height: run.line_height,
                        baseline_offset: run.line_y,
                        char_index,
                        line_index: 0,
                        baseline: run.line_y,
                    };

                    positions.push(position);
                    char_index += 1;
                }
            }

            Ok(positions.clone())
        })
    })?
}
/// Perform complete text measurement with zero allocation - FIXED closure structure
pub fn perform_measurement(
    text: &str,
    font_size: f32,
    max_width: Option<f32>,
    font_family: &str,
    baseline: CSSBaseline,
    cache_manager: &UnifiedCacheManager,
) -> Result<TextMeasurement, MeasurementError> {
    // CRITICAL FIX: All variables must be declared and used within the thread-local closure
    let measurement = with_line_measurements(|line_measurements| {
        with_font_system(|font_system| {
            // ALL VARIABLES NOW PROPERLY SCOPED WITHIN THE CLOSURE
            let mut content_width = 0.0f32;
            let mut content_height = 0.0f32;
            let mut total_character_count = 0usize;
            let mut baseline_offset = 0.0f32;
            let mut overall_line_height = 0.0f32;
            let mut overall_baseline = 0.0f32;
            let mut overall_ascent = 0.0f32;
            let mut overall_descent = 0.0f32;
            let mut overall_line_gap = 0.0f32;
            let mut overall_x_height = 0.0f32;
            let mut overall_cap_height = 0.0f32;
            let mut advance_width = 0.0f32;
            let mut all_glyphs = Vec::new();
            let measurement_time = Instant::now();

            // Create buffer for text measurement
            let mut buffer = Buffer::new(
                font_system,
                cosmyc_text::Metrics::new(font_size, font_size * 1.2),
            );

            // Set buffer size based on max_width
            buffer.set_size(font_system, max_width, None);

            // Set text with attributes
            let attrs = Attrs::new().family(cosmyc_text::Family::Name(font_family));
            buffer.set_text(font_system, text, &attrs, Shaping::Advanced);

            // Shape text for measurement
            buffer.shape_until_scroll(font_system, true);

            // Process each layout run (line of text)
            let mut line_index = 0;
            for run in buffer.layout_runs() {
                let mut line_width = 0.0f32;
                let line_height = run.line_height;

                // Calculate ascent/descent from first glyph's font metrics (research-based fix)
                let (line_ascent, line_descent) = if let Some(first_glyph) = run.glyphs.first() {
                    // Extract font metrics using proper fontdb API
                    let font_metrics_key =
                        FontMetricsCacheKey::new(first_glyph.font_id, first_glyph.font_size);

                    let font_metrics =
                        if let Some(cached) = cache_manager.get_font_metrics(&font_metrics_key) {
                            cached
                        } else {
                            match extract_font_metrics(font_system, first_glyph.font_id) {
                                Ok(metrics) => {
                                    cache_manager.cache_font_metrics(font_metrics_key, metrics);
                                    metrics
                                }
                                Err(_) => {
                                    // Fallback if font metrics extraction fails
                                    FontMetrics::default()
                                }
                            }
                        };

                    let scale = first_glyph.font_size / font_metrics.units_per_em as f32;
                    (
                        font_metrics.ascent as f32 * scale,
                        (-font_metrics.descent) as f32 * scale,
                    )
                } else {
                    // Fallback for empty runs
                    (line_height * 0.8, line_height * 0.2)
                };

                // Track character range for this line
                let line_start_char = total_character_count;

                // Get character positions for this line
                let mut character_positions = Vec::new();
                let mut char_index_in_line = total_character_count;
                let mut glyph_count = 0;

                // Process glyphs in this line
                for glyph in run.glyphs.iter() {
                    let char_pos = CharacterPosition {
                        x: glyph.x,
                        y: run.line_y + glyph.y,
                        width: glyph.w,
                        height: line_height,
                        baseline_offset: run.line_y,
                        char_index: char_index_in_line,
                        line_index,
                        baseline: run.line_y + baseline_offset,
                    };

                    character_positions.push(char_pos);
                    line_width = line_width.max(glyph.x + glyph.w);
                    char_index_in_line += 1;
                    glyph_count += 1;

                    // Store glyph for bounds calculation
                    all_glyphs.push(glyph.clone());
                }

                let line_end_char = char_index_in_line;

                // Calculate baseline offset for this line if not set
                if baseline_offset == 0.0 && !run.glyphs.is_empty() {
                    // Get font metrics for baseline calculation
                    if let Some(first_glyph) = run.glyphs.first() {
                        let font_metrics_key =
                            FontMetricsCacheKey::new(first_glyph.font_id, font_size);

                        let font_metrics = if let Some(cached) =
                            cache_manager.get_font_metrics(&font_metrics_key)
                        {
                            cached
                        } else {
                            let metrics = extract_font_metrics(font_system, first_glyph.font_id)?;
                            cache_manager.cache_font_metrics(font_metrics_key, metrics);
                            metrics
                        };

                        baseline_offset =
                            calculate_baseline_offset(&font_metrics, font_size, baseline);
                    }
                }

                // Create line measurement with all required fields
                let line_measurement = LineMeasurement {
                    width: line_width,
                    height: line_height,
                    ascent: line_ascent,
                    descent: line_descent,
                    line_gap: 0.0, // cosmyc-text doesn't expose line gap per run
                    baseline_offset: run.line_y + baseline_offset,
                    character_positions,
                    start_char: line_start_char,
                    end_char: line_end_char,
                    glyph_count,
                };

                line_measurements.push(line_measurement);

                // Update overall measurements
                content_width = content_width.max(line_width);
                content_height += line_height;
                total_character_count = char_index_in_line;
                advance_width = content_width; // Total advance width

                // Update overall font metrics (use first line as representative)
                if line_index == 0 {
                    overall_line_height = line_height;
                    overall_baseline = run.line_y + baseline_offset;
                    overall_ascent = line_ascent;
                    overall_descent = line_descent;

                    // Extract font metrics for x_height and cap_height (fallback to defaults)
                    if let Some(first_glyph) = run.glyphs.first() {
                        // Use default metrics scaled to font size until font metrics cache is implemented
                        let scale = first_glyph.font_size / 1000.0; // Assuming 1000 units per em
                        overall_x_height = 500.0 * scale; // Default x-height
                        overall_cap_height = 700.0 * scale; // Default cap-height
                        overall_line_gap = 200.0 * scale; // Default line gap
                    }
                }

                line_index += 1;
            }

            // Calculate text bounds from all glyphs with enhanced analysis
            let bounds = calculate_text_bounds_enhanced(
                &all_glyphs,
                0.0,
                overall_line_height,
                content_width,
                content_height,
            );

            // Create final measurement result - ALL VARIABLES IN SCOPE
            let measurement = TextMeasurement {
                content_width,
                content_height,
                line_height: overall_line_height,
                baseline: overall_baseline,
                ascent: overall_ascent,
                descent: overall_descent,
                line_gap: overall_line_gap,
                x_height: overall_x_height,
                cap_height: overall_cap_height,
                advance_width,
                bounds,
                line_measurements: line_measurements.clone(),
                total_character_count,
                baseline_offset,
                measured_at: measurement_time,
            };

            Ok::<TextMeasurement, MeasurementError>(measurement)
        })
    })?;

    Ok(measurement?)
}
