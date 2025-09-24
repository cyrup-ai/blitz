//! Font metrics extraction and baseline calculations
//!
//! This module handles font metrics extraction from the font system,
//! CSS baseline calculations, and font-related caching operations.

use cosmyc_text::{fontdb, FontSystem};

use super::cache::UnifiedCacheManager;
use super::thread_local::with_font_system;
use super::types::*;
use crate::measurement::types::MeasurementError;

/// Calculate CSS baseline offset for a font
pub fn calculate_baseline_offset(
    font_metrics: &FontMetrics,
    font_size: f32,
    baseline: CSSBaseline,
) -> f32 {
    let units_per_em = font_metrics.units_per_em as f32;
    let scale = font_size / units_per_em;

    match baseline {
        CSSBaseline::Alphabetic => 0.0, // Reference baseline
        CSSBaseline::Ideographic => {
            if let Some(ideographic) = font_metrics.ideographic_baseline {
                ideographic as f32 * scale
            } else {
                font_metrics.descent as f32 * scale * 0.8
            }
        }
        CSSBaseline::Hanging => {
            if let Some(hanging) = font_metrics.hanging_baseline {
                hanging as f32 * scale
            } else {
                font_metrics.ascent as f32 * scale * 0.9
            }
        }
        CSSBaseline::Mathematical => {
            if let Some(math_baseline) = font_metrics.mathematical_baseline {
                math_baseline as f32 * scale
            } else if let Some(x_height) = font_metrics.x_height {
                (x_height as f32 * 0.5) * scale
            } else {
                font_metrics.ascent as f32 * scale * 0.35
            }
        }
        CSSBaseline::Central => (font_metrics.ascent + font_metrics.descent) as f32 * scale * 0.5,
        CSSBaseline::Middle => {
            if let Some(x_height) = font_metrics.x_height {
                x_height as f32 * scale * 0.5
            } else {
                font_metrics.ascent as f32 * scale * 0.35
            }
        }
        CSSBaseline::TextTop => font_metrics.ascent as f32 * scale,
        CSSBaseline::TextBottom => font_metrics.descent as f32 * scale,
    }
}

/// Extract font metrics from FontSystem using research-based fontdb API
pub fn extract_font_metrics(
    font_system: &mut FontSystem,
    font_id: fontdb::ID,
) -> Result<FontMetrics, MeasurementError> {
    let mut result = None;

    // Access actual font face data to get metrics
    font_system
        .db_mut()
        .with_face_data(font_id, |font_data, face_index| {
            if let Ok(face) = ttf_parser::Face::parse(font_data, face_index) {
                let units_per_em = face.units_per_em();
                let ascent = face.ascender();
                let descent = face.descender();
                let line_gap = face.line_gap();
                let x_height = face.x_height();
                let cap_height = face.capital_height();

                // Calculate additional baselines using proper font metrics
                let ideographic_baseline = Some((descent as f32 * 0.8) as i16);
                let hanging_baseline = Some((ascent as f32 * 0.9) as i16);
                let mathematical_baseline = x_height.map(|x| (x as f32 * 0.5) as i16);

                // Extract typography metrics for decorations
                let (underline_position, underline_thickness) =
                    if let Some(metrics) = face.underline_metrics() {
                        (metrics.position as f32, metrics.thickness as f32)
                    } else {
                        (-100.0, 50.0)
                    };
                let strikethrough_position = x_height
                    .map(|x| x as f32 * 0.6)
                    .unwrap_or(ascent as f32 * 0.4);
                let strikethrough_thickness = underline_thickness;

                // Calculate character width metrics
                let average_char_width = (ascent - descent) as f32 * 0.5; // Approximation
                let max_char_width = units_per_em as f32; // Maximum possible width

                result = Some(FontMetrics {
                    units_per_em,
                    ascent,
                    descent,
                    line_gap,
                    x_height,
                    cap_height,
                    ideographic_baseline,
                    hanging_baseline,
                    mathematical_baseline,
                    average_char_width,
                    max_char_width,
                    underline_position,
                    underline_thickness,
                    strikethrough_position,
                    strikethrough_thickness,
                });
            }
        });

    result.ok_or_else(|| MeasurementError::FontSystemError)
}

/// Get font metrics with caching
pub fn get_font_metrics(
    font_id: fontdb::ID,
    font_size: f32,
    cache_manager: &UnifiedCacheManager,
) -> Result<FontMetrics, MeasurementError> {
    let key = FontMetricsCacheKey::new(font_id, font_size);

    if let Some(cached) = cache_manager.get_font_metrics(&key) {
        return Ok(cached);
    }

    with_font_system(|font_system| {
        let metrics = extract_font_metrics(font_system, font_id)?;
        cache_manager.cache_font_metrics(key, metrics);
        Ok(metrics)
    })?
}

/// Get baseline offset with caching
pub fn get_baseline_offset(
    font_id: fontdb::ID,
    font_size: f32,
    baseline: CSSBaseline,
    cache_manager: &UnifiedCacheManager,
) -> Result<f32, MeasurementError> {
    let baseline_key = BaselineCacheKey::new(font_id, font_size, baseline);

    if let Some(cached) = cache_manager.get_baseline(&baseline_key) {
        return Ok(cached);
    }

    let font_metrics = get_font_metrics(font_id, font_size, cache_manager)?;
    let offset = calculate_baseline_offset(&font_metrics, font_size, baseline);

    cache_manager.cache_baseline(baseline_key, offset);
    Ok(offset)
}
