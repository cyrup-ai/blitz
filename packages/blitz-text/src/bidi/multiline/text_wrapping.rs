//! Text wrapping and shaped run extraction
//!
//! This module handles text wrapping functionality and extraction of shaped runs
//! for specific lines in multi-line BiDi text processing.

use cosmyc_text::{FontSystem, Metrics};

use super::super::types::{BidiError, BidiRenderOptions, ProcessedBidi};
use crate::types::ShapedRun;

/// Text wrapping processor for multi-line BiDi text
pub struct TextWrapper;

impl TextWrapper {
    /// Create new text wrapper
    pub fn new() -> Self {
        Self
    }

    /// Wrap text to fit within specified width
    pub fn wrap_text(
        processor: &super::core::MultiLineBidiProcessor,
        text: &str,
        options: &BidiRenderOptions,
        shaped_runs: &[ShapedRun],
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<Vec<(ProcessedBidi, Vec<ShapedRun>)>, BidiError> {
        let multiline_result = processor.process_multiline_bidi_text(text, options, font_system, metrics)?;
        let mut wrapped_lines = Vec::new();

        for paragraph in &multiline_result.paragraphs {
            for line in &paragraph.lines {
                // Find shaped runs that correspond to this line
                let line_shaped_runs =
                    Self::extract_shaped_runs_for_line(&line.processed_bidi, shaped_runs)?;

                wrapped_lines.push((line.processed_bidi.clone(), line_shaped_runs));
            }
        }

        Ok(wrapped_lines)
    }

    /// Extract shaped runs that correspond to a specific line
    pub fn extract_shaped_runs_for_line(
        line_bidi: &ProcessedBidi,
        all_shaped_runs: &[ShapedRun],
    ) -> Result<Vec<ShapedRun>, BidiError> {
        let mut line_runs = Vec::new();

        for shaped_run in all_shaped_runs {
            // Check if this shaped run overlaps with the line
            if Self::shaped_run_overlaps_line(shaped_run, line_bidi) {
                // Clone and potentially trim the shaped run to fit the line
                let trimmed_run = Self::trim_shaped_run_to_line(shaped_run, line_bidi)?;
                line_runs.push(trimmed_run);
            }
        }

        Ok(line_runs)
    }

    /// Check if shaped run overlaps with line
    pub fn shaped_run_overlaps_line(shaped_run: &ShapedRun, line_bidi: &ProcessedBidi) -> bool {
        let run_start = shaped_run.start_index;
        let run_end = shaped_run.end_index;

        // For simplicity, assume line covers entire text
        // In practice, would need proper line boundary tracking
        run_start < line_bidi.text.len() && run_end > 0
    }

    /// Trim shaped run to fit within line boundaries
    pub fn trim_shaped_run_to_line(
        shaped_run: &ShapedRun,
        _line_bidi: &ProcessedBidi,
    ) -> Result<ShapedRun, BidiError> {
        // For simplicity, return a clone
        // In practice, would trim glyphs and adjust ranges
        Ok(shaped_run.clone())
    }

    /// Split shaped runs at line boundaries
    pub fn split_shaped_runs_at_boundaries(
        shaped_runs: &[ShapedRun],
        line_boundaries: &[usize],
    ) -> Result<Vec<Vec<ShapedRun>>, BidiError> {
        let mut lines_runs = Vec::new();
        let mut current_line_runs = Vec::new();
        let mut boundary_index = 0;

        for shaped_run in shaped_runs {
            // Check if this run crosses a line boundary
            while boundary_index < line_boundaries.len()
                && shaped_run.start_index >= line_boundaries[boundary_index]
            {
                // Start a new line
                if !current_line_runs.is_empty() {
                    lines_runs.push(current_line_runs);
                    current_line_runs = Vec::new();
                }
                boundary_index += 1;
            }

            // Add run to current line (may need splitting)
            if boundary_index < line_boundaries.len()
                && shaped_run.end_index > line_boundaries[boundary_index]
            {
                // Run crosses boundary - split it
                let (first_part, second_part) = Self::split_shaped_run_at_position(
                    shaped_run,
                    line_boundaries[boundary_index],
                )?;

                current_line_runs.push(first_part);
                lines_runs.push(current_line_runs);
                current_line_runs = vec![second_part];
                boundary_index += 1;
            } else {
                current_line_runs.push(shaped_run.clone());
            }
        }

        // Add final line if not empty
        if !current_line_runs.is_empty() {
            lines_runs.push(current_line_runs);
        }

        Ok(lines_runs)
    }

    /// Split a shaped run at a specific character position
    pub fn split_shaped_run_at_position(
        shaped_run: &ShapedRun,
        split_position: usize,
    ) -> Result<(ShapedRun, ShapedRun), BidiError> {
        // Ensure split position is within run bounds
        if split_position <= shaped_run.start_index || split_position >= shaped_run.end_index {
            return Err(BidiError::InvalidCursorPosition {
                position: split_position,
                text_length: shaped_run.end_index,
            });
        }

        // Create first part (start to split position)
        let first_part = ShapedRun {
            start_index: shaped_run.start_index,
            end_index: split_position,
            ..shaped_run.clone()
        };

        // Create second part (split position to end)
        let second_part = ShapedRun {
            start_index: split_position,
            end_index: shaped_run.end_index,
            ..shaped_run.clone()
        };

        Ok((first_part, second_part))
    }

    /// Merge adjacent shaped runs with compatible properties
    pub fn merge_compatible_runs(shaped_runs: &[ShapedRun]) -> Result<Vec<ShapedRun>, BidiError> {
        if shaped_runs.is_empty() {
            return Ok(Vec::new());
        }

        let mut merged_runs = Vec::new();
        let mut current_run = shaped_runs[0].clone();

        for next_run in shaped_runs.iter().skip(1) {
            if Self::can_merge_runs(&current_run, next_run) {
                // Merge runs by extending the current run's end index
                current_run.end_index = next_run.end_index;
            } else {
                // Cannot merge - add current run and start new one
                merged_runs.push(current_run);
                current_run = next_run.clone();
            }
        }

        // Add the final run
        merged_runs.push(current_run);

        Ok(merged_runs)
    }

    /// Check if two shaped runs can be merged
    pub fn can_merge_runs(first: &ShapedRun, second: &ShapedRun) -> bool {
        // Runs can be merged if:
        // 1. They are adjacent (first.end_index == second.start_index)
        // 2. They have compatible properties (same font, size, etc.)

        first.end_index == second.start_index
        // Add more compatibility checks as needed based on ShapedRun structure
        // For now, assume all adjacent runs can be merged
    }
}

impl Default for TextWrapper {
    fn default() -> Self {
        Self::new()
    }
}
