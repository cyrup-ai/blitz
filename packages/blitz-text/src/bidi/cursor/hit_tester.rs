//! Hit testing for BiDi cursor positioning
//!
//! This module handles hit testing functionality, converting visual coordinates
//! to logical text positions with proper BiDi support.

use super::super::types::{BidiError, CursorPosition, ProcessedBidi, VisualRun};
use super::position_calculator::PositionCalculator;
use crate::types::ShapedRun;

/// Hit tester for converting visual coordinates to logical positions
pub struct HitTester {
    position_calculator: PositionCalculator,
}

impl HitTester {
    /// Create new hit tester
    pub fn new(position_calculator: PositionCalculator) -> Self {
        Self {
            position_calculator,
        }
    }

    /// Hit test: convert visual coordinates to logical text position
    pub fn hit_test(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        x: f32,
        y: f32,
        line_height: f32,
    ) -> Result<CursorPosition, BidiError> {
        let line_index = (y / line_height).floor() as usize;

        // Find the visual run at the given x coordinate
        let mut current_x = 0.0;

        for visual_run in &processed_bidi.visual_runs {
            let run_width = self
                .position_calculator
                .calculate_run_width_from_shaped(visual_run, shaped_runs)?;

            if x >= current_x && x < current_x + run_width {
                // Hit is within this run
                let relative_x = x - current_x;
                let logical_index =
                    self.find_logical_index_in_run(visual_run, shaped_runs, relative_x)?;

                return self.position_calculator.calculate_cursor_position_uncached(
                    processed_bidi,
                    shaped_runs,
                    logical_index,
                    line_index,
                );
            }

            current_x += run_width;
        }

        // Hit is beyond the end of text
        self.position_calculator.get_end_of_text_cursor_position(
            processed_bidi,
            shaped_runs,
            line_index,
        )
    }

    /// Find logical index within a visual run at given x coordinate
    fn find_logical_index_in_run(
        &self,
        visual_run: &VisualRun,
        shaped_runs: &[ShapedRun],
        x: f32,
    ) -> Result<usize, BidiError> {
        for shaped_run in shaped_runs {
            if self
                .position_calculator
                .runs_overlap(visual_run, shaped_run)
            {
                let mut current_x = 0.0;

                for glyph in &shaped_run.glyphs {
                    if (glyph.cluster as usize) >= visual_run.start_index
                        && (glyph.cluster as usize) < visual_run.end_index
                    {
                        let glyph_width = glyph.x_advance;

                        if x >= current_x && x < current_x + glyph_width {
                            // Determine if hit is in leading or trailing half
                            let hit_offset = x - current_x;
                            if hit_offset < glyph_width / 2.0 {
                                return Ok(glyph.cluster as usize);
                            } else {
                                return Ok((glyph.cluster + 1) as usize);
                            }
                        }

                        current_x += glyph_width;
                    }
                }
            }
        }

        // Default to start of run
        Ok(visual_run.start_index)
    }

    /// Hit test with detailed information about the hit
    pub fn hit_test_detailed(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        x: f32,
        y: f32,
        line_height: f32,
    ) -> Result<HitTestResult, BidiError> {
        let cursor_position = self.hit_test(processed_bidi, shaped_runs, x, y, line_height)?;

        // Find which visual run contains the hit
        let visual_run = self
            .position_calculator
            .find_visual_run_for_logical_index(processed_bidi, cursor_position.logical_index)
            .ok();

        // Calculate distance from character boundaries
        let character_info = self.get_character_boundaries(
            processed_bidi,
            shaped_runs,
            cursor_position.logical_index,
        )?;

        Ok(HitTestResult {
            cursor_position,
            visual_run: visual_run.cloned(),
            character_boundaries: character_info,
            hit_coordinates: HitCoordinates { x, y },
        })
    }

    /// Get character boundary information for a logical index
    fn get_character_boundaries(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        logical_index: usize,
    ) -> Result<CharacterBoundaries, BidiError> {
        if logical_index >= processed_bidi.text.chars().count() {
            return Ok(CharacterBoundaries {
                leading_edge: 0.0,
                trailing_edge: 0.0,
                width: 0.0,
            });
        }

        let visual_run = self
            .position_calculator
            .find_visual_run_for_logical_index(processed_bidi, logical_index)?;

        let leading_x = self.position_calculator.calculate_visual_x_position(
            processed_bidi,
            shaped_runs,
            visual_run,
            logical_index,
        )?;

        let trailing_x = if logical_index + 1 < visual_run.end_index {
            self.position_calculator.calculate_visual_x_position(
                processed_bidi,
                shaped_runs,
                visual_run,
                logical_index + 1,
            )?
        } else {
            leading_x // At end of run
        };

        Ok(CharacterBoundaries {
            leading_edge: leading_x,
            trailing_edge: trailing_x,
            width: (trailing_x - leading_x).abs(),
        })
    }
}

/// Detailed hit test result
#[derive(Debug, Clone)]
pub struct HitTestResult {
    pub cursor_position: CursorPosition,
    pub visual_run: Option<VisualRun>,
    pub character_boundaries: CharacterBoundaries,
    pub hit_coordinates: HitCoordinates,
}

/// Character boundary information
#[derive(Debug, Clone)]
pub struct CharacterBoundaries {
    pub leading_edge: f32,
    pub trailing_edge: f32,
    pub width: f32,
}

/// Hit test coordinates
#[derive(Debug, Clone)]
pub struct HitCoordinates {
    pub x: f32,
    pub y: f32,
}
