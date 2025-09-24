//! Utility functions and helpers
//!
//! This module provides utility functions for cursor manipulation, color operations,
//! and metrics calculations.

use cosmyc_text::{Affinity, Color, Cursor, FontSystem, Metrics, Motion};

use super::buffer::EnhancedBuffer;

/// Cursor utilities and extensions
pub struct CursorUtils;

impl CursorUtils {
    /// Create cursor from line and column
    pub fn from_line_col(line: usize, col: usize) -> Cursor {
        Cursor::new(line, col)
    }

    /// Create cursor with affinity
    pub fn with_affinity(line: usize, index: usize, affinity: Affinity) -> Cursor {
        Cursor::new_with_affinity(line, index, affinity)
    }

    /// Check if cursor is at start of line
    pub fn is_line_start(cursor: &Cursor) -> bool {
        cursor.index == 0
    }

    /// Move cursor to next grapheme cluster
    pub fn next_cluster(
        buffer: &mut EnhancedBuffer,
        font_system: &mut FontSystem,
        cursor: Cursor,
    ) -> Option<Cursor> {
        buffer
            .move_cursor(font_system, cursor, None, Motion::Right)
            .map(|(cursor, _)| cursor)
    }

    /// Move cursor to previous grapheme cluster  
    pub fn prev_cluster(
        buffer: &mut EnhancedBuffer,
        font_system: &mut FontSystem,
        cursor: Cursor,
    ) -> Option<Cursor> {
        buffer
            .move_cursor(font_system, cursor, None, Motion::Left)
            .map(|(cursor, _)| cursor)
    }

    /// Move cursor to line start
    pub fn line_start(
        buffer: &mut EnhancedBuffer,
        font_system: &mut FontSystem,
        cursor: Cursor,
    ) -> Option<Cursor> {
        buffer
            .move_cursor(font_system, cursor, None, Motion::Home)
            .map(|(cursor, _)| cursor)
    }

    /// Move cursor to line end
    pub fn line_end(
        buffer: &mut EnhancedBuffer,
        font_system: &mut FontSystem,
        cursor: Cursor,
    ) -> Option<Cursor> {
        buffer
            .move_cursor(font_system, cursor, None, Motion::End)
            .map(|(cursor, _)| cursor)
    }
}

/// Color utilities and extensions
pub struct ColorUtils;

impl ColorUtils {
    /// Create RGB color
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::rgb(r, g, b)
    }

    /// Create RGBA color
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::rgba(r, g, b, a)
    }

    /// Convert color to RGBA tuple
    pub fn to_rgba_tuple(color: Color) -> (u8, u8, u8, u8) {
        color.as_rgba_tuple()
    }

    /// Convert color to RGBA array
    pub fn to_rgba_array(color: Color) -> [u8; 4] {
        color.as_rgba()
    }

    /// Blend two colors with alpha
    pub fn blend_alpha(base: Color, overlay: Color) -> Color {
        let base_rgba = base.as_rgba();
        let overlay_rgba = overlay.as_rgba();

        let alpha_overlay = overlay_rgba[3] as f32 / 255.0;
        let alpha_base = base_rgba[3] as f32 / 255.0;
        let alpha_result = alpha_overlay + alpha_base * (1.0 - alpha_overlay);

        if alpha_result == 0.0 {
            return Color::rgba(0, 0, 0, 0);
        }

        let r = ((overlay_rgba[0] as f32 * alpha_overlay
            + base_rgba[0] as f32 * alpha_base * (1.0 - alpha_overlay))
            / alpha_result) as u8;
        let g = ((overlay_rgba[1] as f32 * alpha_overlay
            + base_rgba[1] as f32 * alpha_base * (1.0 - alpha_overlay))
            / alpha_result) as u8;
        let b = ((overlay_rgba[2] as f32 * alpha_overlay
            + base_rgba[2] as f32 * alpha_base * (1.0 - alpha_overlay))
            / alpha_result) as u8;
        let a = (alpha_result * 255.0) as u8;

        Color::rgba(r, g, b, a)
    }
}

/// Metrics utilities and extensions
pub struct MetricsUtils;

impl MetricsUtils {
    /// Create metrics with font size and line height
    pub const fn new(font_size: f32, line_height: f32) -> Metrics {
        Metrics::new(font_size, line_height)
    }

    /// Create metrics with relative line height
    pub fn relative(font_size: f32, line_height_scale: f32) -> Metrics {
        Metrics::relative(font_size, line_height_scale)
    }

    /// Scale metrics by factor
    pub fn scale(metrics: Metrics, scale: f32) -> Metrics {
        metrics.scale(scale)
    }

    /// Calculate line height from font size and scale
    pub fn calculate_line_height(font_size: f32, scale: f32) -> f32 {
        font_size * scale
    }
}
