//! Selection management for BiDi text
//!
//! This module handles text selection operations including range creation,
//! validation, and visual representation with proper BiDi support.

use std::cmp::{max, min};

use super::super::types::{BidiError, CursorPosition, ProcessedBidi, Selection};
use super::position_calculator::PositionCalculator;
use crate::types::ShapedRun;

/// Selection manager for BiDi text operations
pub struct SelectionManager {
    position_calculator: PositionCalculator,
}

impl SelectionManager {
    /// Create new selection manager
    pub fn new(position_calculator: PositionCalculator) -> Self {
        Self {
            position_calculator,
        }
    }

    /// Create selection between two logical positions
    pub fn create_selection(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        start_logical: usize,
        end_logical: usize,
        line_index: usize,
    ) -> Result<Selection, BidiError> {
        let text_length = processed_bidi.text.chars().count();

        if start_logical > text_length || end_logical > text_length {
            return Err(BidiError::InvalidCursorPosition {
                position: max(start_logical, end_logical),
                text_length,
            });
        }

        let start_position = self
            .position_calculator
            .calculate_cursor_position_uncached(
                processed_bidi,
                shaped_runs,
                start_logical,
                line_index,
            )?;

        let end_position = self
            .position_calculator
            .calculate_cursor_position_uncached(
                processed_bidi,
                shaped_runs,
                end_logical,
                line_index,
            )?;

        // Determine visual ordering
        let (visual_start, visual_end) = if start_position.visual_x <= end_position.visual_x {
            (start_position, end_position)
        } else {
            (end_position, start_position)
        };

        Ok(Selection {
            logical_start: min(start_logical, end_logical),
            logical_end: max(start_logical, end_logical),
            visual_start,
            visual_end,
            is_empty: start_logical == end_logical,
        })
    }

    /// Extend selection from current position to new position
    pub fn extend_selection(
        &self,
        current_selection: &Selection,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        new_logical_end: usize,
        line_index: usize,
    ) -> Result<Selection, BidiError> {
        self.create_selection(
            processed_bidi,
            shaped_runs,
            current_selection.logical_start,
            new_logical_end,
            line_index,
        )
    }

    /// Get selected text from selection
    pub fn get_selected_text(
        &self,
        processed_bidi: &ProcessedBidi,
        selection: &Selection,
    ) -> String {
        if selection.is_empty {
            return String::new();
        }

        let chars: Vec<char> = processed_bidi.text.chars().collect();
        let start = min(selection.logical_start, chars.len());
        let end = min(selection.logical_end, chars.len());

        chars[start..end].iter().collect()
    }

    /// Check if selection contains a logical position
    pub fn selection_contains(&self, selection: &Selection, logical_index: usize) -> bool {
        if selection.is_empty {
            return false;
        }

        logical_index >= selection.logical_start && logical_index < selection.logical_end
    }

    /// Get visual rectangles for selection rendering
    pub fn get_selection_rectangles(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        selection: &Selection,
        line_height: f32,
    ) -> Result<Vec<SelectionRectangle>, BidiError> {
        if selection.is_empty {
            return Ok(Vec::new());
        }

        let mut rectangles = Vec::new();
        let mut current_rect: Option<SelectionRectangle> = None;

        // Process each character in the logical selection range
        for logical_index in selection.logical_start..selection.logical_end {
            let position = self
                .position_calculator
                .calculate_cursor_position_uncached(
                    processed_bidi,
                    shaped_runs,
                    logical_index,
                    selection.visual_start.line_index,
                )?;

            let next_position = if logical_index + 1 < selection.logical_end {
                self.position_calculator
                    .calculate_cursor_position_uncached(
                        processed_bidi,
                        shaped_runs,
                        logical_index + 1,
                        selection.visual_start.line_index,
                    )?
            } else {
                // Last character, use trailing edge
                CursorPosition {
                    visual_x: position.visual_x
                        + self.estimate_character_width(
                            processed_bidi,
                            shaped_runs,
                            logical_index,
                        )?,
                    ..position
                }
            };

            let char_left = min_f32(position.visual_x, next_position.visual_x);
            let char_right = max_f32(position.visual_x, next_position.visual_x);

            // Try to merge with current rectangle if adjacent
            if let Some(ref mut rect) = current_rect {
                if (char_left - rect.right).abs() < 1.0 && rect.line_index == position.line_index {
                    // Extend current rectangle
                    rect.right = char_right;
                } else {
                    // Start new rectangle
                    rectangles.push(rect.clone());
                    current_rect = Some(SelectionRectangle {
                        left: char_left,
                        right: char_right,
                        top: position.line_index as f32 * line_height,
                        bottom: (position.line_index + 1) as f32 * line_height,
                        line_index: position.line_index,
                    });
                }
            } else {
                // First rectangle
                current_rect = Some(SelectionRectangle {
                    left: char_left,
                    right: char_right,
                    top: position.line_index as f32 * line_height,
                    bottom: (position.line_index + 1) as f32 * line_height,
                    line_index: position.line_index,
                });
            }
        }

        // Add final rectangle
        if let Some(rect) = current_rect {
            rectangles.push(rect);
        }

        Ok(rectangles)
    }

    /// Estimate character width for selection rectangle calculation
    fn estimate_character_width(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        logical_index: usize,
    ) -> Result<f32, BidiError> {
        let visual_run = self
            .position_calculator
            .find_visual_run_for_logical_index(processed_bidi, logical_index)?;

        // Find corresponding shaped run and glyph
        for shaped_run in shaped_runs {
            if self
                .position_calculator
                .runs_overlap(visual_run, shaped_run)
            {
                for glyph in &shaped_run.glyphs {
                    if glyph.cluster as usize == logical_index {
                        return Ok(glyph.x_advance);
                    }
                }
            }
        }

        // Default fallback width
        Ok(8.0)
    }
}

/// Rectangle representing a visual selection area
#[derive(Debug, Clone)]
pub struct SelectionRectangle {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub line_index: usize,
}

/// Helper function for minimum of two f32 values
fn min_f32(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

/// Helper function for maximum of two f32 values
fn max_f32(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}
