//! Core cursor manager for BiDi text
//!
//! This module provides the main CursorManager interface that coordinates
//! position calculation, hit testing, selection management, and statistics.

use std::sync::Arc;

use super::super::types::{
    BidiError, BidiSelection, CursorPosition, Direction, ProcessedBidi, Selection, SelectionRect,
};
use super::hit_tester::{HitTestResult, HitTester};
use super::position_calculator::PositionCalculator;
use super::selection_manager::{SelectionManager, SelectionRectangle};
use super::types::CursorStats;
use crate::types::ShapedRun;

/// Calculate selection rectangles for bidirectional text
fn calculate_selection_rectangles(
    selection: &Selection,
    processed_bidi: &ProcessedBidi,
    shaped_runs: &[ShapedRun],
    line_index: usize,
) -> Result<Vec<SelectionRectangle>, BidiError> {
    let mut rectangles = Vec::new();

    // Handle logical vs visual position mapping
    let logical_start = selection.logical_start;
    let logical_end = selection.logical_end;

    // Map logical positions to visual runs
    for visual_run in &processed_bidi.visual_runs {
        let overlap_start = logical_start.max(visual_run.start_index);
        let overlap_end = logical_end.min(visual_run.end_index);

        if overlap_start < overlap_end {
            // Calculate visual bounds for this segment
            let _run_start_x = calculate_visual_x_position(visual_run.start_index, shaped_runs)?;
            let selection_start_x = calculate_visual_x_position(overlap_start, shaped_runs)?;
            let selection_end_x = calculate_visual_x_position(overlap_end, shaped_runs)?;

            // Handle RTL runs (reverse X coordinates)
            let (rect_x, rect_width) = if visual_run.direction == Direction::RightToLeft {
                let x = selection_end_x;
                let width = selection_start_x - selection_end_x;
                (x, width)
            } else {
                let x = selection_start_x;
                let width = selection_end_x - selection_start_x;
                (x, width)
            };

            // Use actual line metrics from shaped runs
            let line_height = shaped_runs
                .first()
                .map(|run| run.height)
                .unwrap_or(20.0); // Fallback to 20.0 if no runs available
            
            let rect_y = (line_index as f32) * line_height;
            let rect_height = line_height;

            rectangles.push(SelectionRectangle {
                left: rect_x,
                right: rect_x + rect_width,
                top: rect_y,
                bottom: rect_y + rect_height,
                line_index,
            });
        }
    }

    Ok(rectangles)
}

/// Calculate visual X position for logical character index
fn calculate_visual_x_position(
    logical_index: usize,
    shaped_runs: &[ShapedRun],
) -> Result<f32, BidiError> {
    for run in shaped_runs {
        if logical_index >= run.start_index && logical_index < run.end_index {
            let run_offset = logical_index - run.start_index;

            // Accumulate glyph advances up to the target position
            let mut x = 0.0;
            for (_i, glyph) in run.glyphs.iter().enumerate() {
                if (glyph.cluster as usize - run.start_index) >= run_offset {
                    break;
                }
                x += glyph.x_advance;
            }

            return Ok(x);
        }
    }

    Err(BidiError::InvalidCursorPosition {
        position: logical_index,
        text_length: shaped_runs.len(),
    })
}

/// Main cursor manager for BiDi text operations
pub struct CursorManager {
    position_calculator: PositionCalculator,
    hit_tester: HitTester,
    selection_manager: SelectionManager,
    stats: CursorStats,
}

impl CursorManager {
    /// Create new cursor manager with default direction
    pub fn new(default_direction: Direction) -> Self {
        let position_calculator = PositionCalculator::new(default_direction);
        let hit_tester = HitTester::new(PositionCalculator::new(default_direction));
        let selection_manager = SelectionManager::new(PositionCalculator::new(default_direction));
        let stats = CursorStats::new();

        Self {
            position_calculator,
            hit_tester,
            selection_manager,
            stats,
        }
    }

    /// Get cursor position from logical text index with caching
    pub fn get_cursor_position(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        logical_index: usize,
        line_index: usize,
    ) -> Result<Arc<CursorPosition>, BidiError> {
        self.position_calculator.get_cursor_position(
            processed_bidi,
            shaped_runs,
            logical_index,
            line_index,
        )
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
        self.hit_tester
            .hit_test(processed_bidi, shaped_runs, x, y, line_height)
    }

    /// Hit test with detailed information
    pub fn hit_test_detailed(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        x: f32,
        y: f32,
        line_height: f32,
    ) -> Result<HitTestResult, BidiError> {
        self.hit_tester
            .hit_test_detailed(processed_bidi, shaped_runs, x, y, line_height)
    }

    /// Create selection between two logical positions
    pub fn create_selection(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        start_logical: usize,
        end_logical: usize,
        line_index: usize,
    ) -> Result<BidiSelection, BidiError> {
        let selection = self.selection_manager.create_selection(
            processed_bidi,
            shaped_runs,
            start_logical,
            end_logical,
            line_index,
        )?;

        // Convert Selection to BidiSelection
        let selection_rectangles =
            calculate_selection_rectangles(&selection, processed_bidi, shaped_runs, line_index)?;

        // Convert SelectionRectangle to SelectionRect for compatibility
        let rectangles = selection_rectangles
            .iter()
            .map(|rect| SelectionRect {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
                direction: Direction::LeftToRight, // Default direction
            })
            .collect();

        Ok(BidiSelection {
            position: selection.logical_start,
            length: selection.logical_end - selection.logical_start,
            rectangles,
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
    ) -> Result<BidiSelection, BidiError> {
        let selection = self.selection_manager.extend_selection(
            current_selection,
            processed_bidi,
            shaped_runs,
            new_logical_end,
            line_index,
        )?;

        // Convert Selection to BidiSelection
        let selection_rectangles =
            calculate_selection_rectangles(&selection, processed_bidi, shaped_runs, line_index)?;

        // Convert SelectionRectangle to SelectionRect for compatibility
        let rectangles = selection_rectangles
            .iter()
            .map(|rect| SelectionRect {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
                direction: Direction::LeftToRight, // Default direction
            })
            .collect();

        Ok(BidiSelection {
            position: selection.logical_start,
            length: selection.logical_end - selection.logical_start,
            rectangles,
        })
    }

    /// Get selected text from selection
    pub fn get_selected_text(
        &self,
        processed_bidi: &ProcessedBidi,
        selection: &Selection,
    ) -> String {
        self.selection_manager
            .get_selected_text(processed_bidi, selection)
    }

    /// Check if selection contains a logical position
    pub fn selection_contains(&self, selection: &Selection, logical_index: usize) -> bool {
        self.selection_manager
            .selection_contains(selection, logical_index)
    }

    /// Get visual rectangles for selection rendering
    pub fn get_selection_rectangles(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        selection: &Selection,
        line_height: f32,
    ) -> Result<Vec<SelectionRectangle>, BidiError> {
        self.selection_manager.get_selection_rectangles(
            processed_bidi,
            shaped_runs,
            selection,
            line_height,
        )
    }

    /// Get cursor statistics
    pub fn get_stats(&self) -> &CursorStats {
        &self.stats
    }

    /// Reset cursor statistics
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Update statistics after cache operation
    pub fn update_cache_stats(&mut self, was_cache_hit: bool) {
        self.stats.update_cache_stats(was_cache_hit);
    }
}
