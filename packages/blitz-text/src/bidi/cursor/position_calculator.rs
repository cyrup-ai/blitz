//! Cursor position calculation for BiDi text
//!
//! This module handles the calculation of cursor positions from logical text indices,
//! including visual coordinate mapping and end-of-text positioning.

use std::sync::Arc;

use super::super::cache::CacheManager;
use super::super::types::{
    BidiError, CursorCacheKey, CursorPosition, Direction, ProcessedBidi, VisualRun,
};
use crate::types::ShapedRun;

/// Position calculator for cursor operations
pub struct PositionCalculator {
    default_direction: Direction,
}

impl PositionCalculator {
    /// Create new position calculator
    pub fn new(default_direction: Direction) -> Self {
        Self { default_direction }
    }

    /// Get cursor position from logical text index with caching
    pub fn get_cursor_position(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        logical_index: usize,
        line_index: usize,
    ) -> Result<Arc<CursorPosition>, BidiError> {
        // Create cache key
        let cache_key = CursorCacheKey::new(
            &processed_bidi.text,
            logical_index,
            processed_bidi.base_direction,
        );

        // Try cache first
        if let Some(cached) = CacheManager::get_cursor_cached(&cache_key) {
            return Ok(cached);
        }

        // Calculate cursor position
        let position = self.calculate_cursor_position_uncached(
            processed_bidi,
            shaped_runs,
            logical_index,
            line_index,
        )?;

        let arc_position = Arc::new(position);

        // Store in cache
        CacheManager::store_cursor_cached(cache_key, arc_position.clone());

        Ok(arc_position)
    }

    /// Calculate cursor position without caching
    pub fn calculate_cursor_position_uncached(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        logical_index: usize,
        line_index: usize,
    ) -> Result<CursorPosition, BidiError> {
        if logical_index > processed_bidi.text.chars().count() {
            return Err(BidiError::InvalidCursorPosition {
                position: logical_index,
                text_length: processed_bidi.text.chars().count(),
            });
        }

        // Handle edge case: cursor at end of text
        if logical_index == processed_bidi.text.chars().count() {
            return self.get_end_of_text_cursor_position(processed_bidi, shaped_runs, line_index);
        }

        // Find the visual run containing this logical index
        let visual_run = self.find_visual_run_for_logical_index(processed_bidi, logical_index)?;

        // Calculate visual x position within the run
        let visual_x = self.calculate_visual_x_position(
            processed_bidi,
            shaped_runs,
            &visual_run,
            logical_index,
        )?;

        // Determine cursor direction and level
        let direction = visual_run.direction;
        let level = visual_run.level;

        // Check if cursor is at trailing edge of character
        let is_trailing = self.is_cursor_trailing(processed_bidi, logical_index, &visual_run);

        Ok(CursorPosition {
            logical_index,
            visual_x,
            line_index,
            is_trailing,
            direction,
            level,
        })
    }

    /// Get cursor position at end of text
    pub fn get_end_of_text_cursor_position(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        line_index: usize,
    ) -> Result<CursorPosition, BidiError> {
        let text_length = processed_bidi.text.chars().count();

        // Calculate total width of all visual runs
        let mut total_width = 0.0;
        for shaped_run in shaped_runs {
            for glyph in &shaped_run.glyphs {
                total_width += glyph.x_advance;
            }
        }

        Ok(CursorPosition {
            logical_index: text_length,
            visual_x: total_width,
            line_index,
            is_trailing: true,
            direction: processed_bidi.base_direction,
            level: processed_bidi.paragraph_level,
        })
    }

    /// Find visual run containing the given logical index
    pub fn find_visual_run_for_logical_index<'a>(
        &self,
        processed_bidi: &'a ProcessedBidi,
        logical_index: usize,
    ) -> Result<&'a VisualRun, BidiError> {
        for visual_run in &processed_bidi.visual_runs {
            if logical_index >= visual_run.start_index && logical_index < visual_run.end_index {
                return Ok(visual_run);
            }
        }

        Err(BidiError::InvalidCursorPosition {
            position: logical_index,
            text_length: processed_bidi.text.chars().count(),
        })
    }

    /// Calculate visual x position for cursor within a run
    pub fn calculate_visual_x_position(
        &self,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        visual_run: &VisualRun,
        logical_index: usize,
    ) -> Result<f32, BidiError> {
        let mut visual_x = 0.0;

        // Add width of all previous visual runs
        for prev_run in &processed_bidi.visual_runs {
            if prev_run.visual_order < visual_run.visual_order {
                visual_x += self.calculate_run_width_from_shaped(prev_run, shaped_runs)?;
            }
        }

        // Add width within current run up to logical index
        let run_offset = logical_index - visual_run.start_index;
        visual_x += self.calculate_partial_run_width(visual_run, shaped_runs, run_offset)?;

        Ok(visual_x)
    }

    /// Calculate width of a visual run from shaped runs
    pub fn calculate_run_width_from_shaped(
        &self,
        visual_run: &VisualRun,
        shaped_runs: &[ShapedRun],
    ) -> Result<f32, BidiError> {
        for shaped_run in shaped_runs {
            if self.runs_overlap(visual_run, shaped_run) {
                let mut width = 0.0;
                for glyph in &shaped_run.glyphs {
                    if (glyph.cluster as usize) >= visual_run.start_index
                        && (glyph.cluster as usize) < visual_run.end_index
                    {
                        width += glyph.x_advance;
                    }
                }
                return Ok(width);
            }
        }
        Ok(0.0)
    }

    /// Calculate partial width within a run up to a specific offset
    fn calculate_partial_run_width(
        &self,
        visual_run: &VisualRun,
        shaped_runs: &[ShapedRun],
        char_offset: usize,
    ) -> Result<f32, BidiError> {
        for shaped_run in shaped_runs {
            if self.runs_overlap(visual_run, shaped_run) {
                let mut width = 0.0;
                let mut chars_processed = 0;

                for glyph in &shaped_run.glyphs {
                    if (glyph.cluster as usize) >= visual_run.start_index
                        && (glyph.cluster as usize) < visual_run.end_index
                    {
                        if chars_processed >= char_offset {
                            break;
                        }
                        width += glyph.x_advance;
                        chars_processed += 1;
                    }
                }
                return Ok(width);
            }
        }
        Ok(0.0)
    }

    /// Check if visual run and shaped run overlap
    pub fn runs_overlap(&self, visual_run: &VisualRun, shaped_run: &ShapedRun) -> bool {
        let visual_start = visual_run.start_index;
        let visual_end = visual_run.end_index;
        let shaped_start = shaped_run.start_index;
        let shaped_end = shaped_run.end_index;

        visual_start < shaped_end && visual_end > shaped_start
    }

    /// Check if cursor is at trailing edge of character
    fn is_cursor_trailing(
        &self,
        _processed_bidi: &ProcessedBidi,
        logical_index: usize,
        visual_run: &VisualRun,
    ) -> bool {
        // For RTL text, cursor positioning logic is different
        match visual_run.direction {
            Direction::RightToLeft => {
                // In RTL, trailing edge is visually to the left
                logical_index > visual_run.start_index
            }
            Direction::LeftToRight => {
                // In LTR, trailing edge is visually to the right
                logical_index > visual_run.start_index
            }
            Direction::Auto => false,
        }
    }
}
