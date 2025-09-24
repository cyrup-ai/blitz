//! Bounds and position type definitions
//!
//! This module contains types for representing text bounds, character positions,
//! and spatial measurements in the text layout system.

use serde::{Deserialize, Serialize};

/// Ink bounds (actual glyph pixels)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InkBounds {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl Default for InkBounds {
    fn default() -> Self {
        Self {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 0.0,
            y_max: 0.0,
        }
    }
}

/// Logical bounds (font metrics)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogicalBounds {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl Default for LogicalBounds {
    fn default() -> Self {
        Self {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 0.0,
            y_max: 0.0,
        }
    }
}

/// Text bounds with ink and logical bounds
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TextBounds {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub ink_bounds: InkBounds,
    pub logical_bounds: LogicalBounds,
}

impl Default for TextBounds {
    fn default() -> Self {
        Self {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 0.0,
            y_max: 0.0,
            ink_bounds: InkBounds::default(),
            logical_bounds: LogicalBounds::default(),
        }
    }
}

/// Position of a character in text layout
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CharacterPosition {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub baseline_offset: f32,
    pub char_index: usize,
    pub line_index: usize,
    pub baseline: f32,
}

impl Default for CharacterPosition {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            baseline_offset: 0.0,
            char_index: 0,
            line_index: 0,
            baseline: 0.0,
        }
    }
}
