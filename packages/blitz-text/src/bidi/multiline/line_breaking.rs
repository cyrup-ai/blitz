//! Line breaking algorithms and width calculations
//!
//! This module handles Unicode line break opportunities, width constraints,
//! and line break decision logic for multi-line BiDi text.

use unicode_linebreak::{linebreaks, BreakOpportunity};

use cosmyc_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

use super::super::types::{BidiError, LineBreakInfo, ProcessedBidi};
use crate::shaping::types::{ShapedRun, TextDirection};

/// Line breaking processor for multi-line text
pub struct LineBreaker {
    max_width: f32,
}

impl LineBreaker {
    /// Create new line breaker with maximum width
    pub fn new(max_width: f32) -> Self {
        Self { max_width }
    }

    /// Set maximum line width
    pub fn set_max_width(&mut self, max_width: f32) {
        self.max_width = max_width;
    }

    /// Get current maximum width
    pub fn max_width(&self) -> f32 {
        self.max_width
    }

    /// Calculate line breaks for text
    pub fn calculate_line_breaks(
        &self,
        text: &str,
        processed_bidi: &ProcessedBidi,
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<Vec<LineBreakInfo>, BidiError> {
        // Find Unicode line break opportunities
        let break_opportunities: Vec<_> = linebreaks(text).collect();
        let mut lines = Vec::new();
        let mut current_line_start = 0;

        for (_break_index, break_opportunity) in break_opportunities.iter().enumerate() {
            let break_position = break_opportunity.0;

            // Check if we should break here based on width constraints
            if self.should_break_line(text, current_line_start, break_position, processed_bidi, font_system, metrics)? {
                let line_text = &text[current_line_start..break_position];
                let line_width = self.calculate_line_width(line_text, processed_bidi, font_system, metrics)?;

                lines.push(LineBreakInfo {
                    text: line_text.to_string(),
                    break_positions: vec![break_position],
                    break_opportunities: vec![matches!(
                        break_opportunity.1,
                        BreakOpportunity::Allowed
                    )],
                    line_widths: vec![line_width],
                    max_width: self.max_width,
                });

                current_line_start = break_position;
            }
        }

        // Handle remaining text
        if current_line_start < text.len() {
            let line_text = &text[current_line_start..];
            let line_width = self.calculate_line_width(line_text, processed_bidi, font_system, metrics)?;

            lines.push(LineBreakInfo {
                text: line_text.to_string(),
                break_positions: vec![text.len()],
                break_opportunities: vec![true],
                line_widths: vec![line_width],
                max_width: self.max_width,
            });
        }

        Ok(lines)
    }

    /// Check if line should be broken at given position
    pub fn should_break_line(
        &self,
        text: &str,
        line_start: usize,
        break_position: usize,
        processed_bidi: &ProcessedBidi,
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<bool, BidiError> {
        let line_text = &text[line_start..break_position];
        let line_width = self.calculate_line_width(
            line_text,
            processed_bidi,
            font_system,
            metrics,
        )?;

        Ok(line_width > self.max_width)
    }

    /// Calculate width of a line of text
    pub fn calculate_line_width(
        &self,
        line_text: &str,
        processed_bidi: &ProcessedBidi,
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<f32, BidiError> {
        // Handle empty text edge case
        if line_text.is_empty() {
            return Ok(0.0);
        }

        // Create temporary buffer for accurate measurement
        let mut temp_buffer = Buffer::new(font_system, metrics);
        temp_buffer.set_text(font_system, line_text, &Attrs::new(), Shaping::Advanced);
        temp_buffer.set_size(font_system, Some(f32::INFINITY), None);

        // Calculate accurate width from shaped runs
        let width = temp_buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0f32, f32::max);

        Ok(width)
    }

    /// Find optimal break points using advanced algorithms
    pub fn find_optimal_breaks(
        &self,
        text: &str,
        processed_bidi: &ProcessedBidi,
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<Vec<usize>, BidiError> {
        let _ = text; // Unused parameter, kept for API compatibility
        let mut optimal_breaks = Vec::new();

        // For each visual run, find break opportunities directly from text
        for visual_run in &processed_bidi.visual_runs {
            // Get Unicode line break opportunities directly from the text
            // This avoids the glyph reconstruction issue
            let break_positions: Vec<usize> = linebreaks(&visual_run.text)
                .map(|(pos, _)| visual_run.start_index + pos)
                .collect();

            // Shape the text to get glyph metrics for width calculation
            let temp_run = self.create_shaped_run_from_visual(visual_run, font_system, metrics)?;

            // Select breaks based on width constraints
            let selected = self.select_breaks_by_width(
                &break_positions,
                &temp_run,
                visual_run.start_index,
            )?;

            optimal_breaks.extend(selected);
        }

        // Sort breaks by position
        optimal_breaks.sort_unstable();
        optimal_breaks.dedup();

        Ok(optimal_breaks)
    }

    /// Calculate penalty for breaking at a specific position
    pub fn calculate_break_penalty(
        &self,
        text: &str,
        break_position: usize,
        processed_bidi: &ProcessedBidi,
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<f32, BidiError> {
        // Penalty factors:
        // - Line length deviation from ideal
        // - Breaking at bad positions (e.g., after punctuation)
        // - Hyphenation requirements

        let line_start = text[..break_position]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);
        let line_text = &text[line_start..break_position];
        let line_width = self.calculate_line_width(line_text, processed_bidi, font_system, metrics)?;

        // Simple penalty based on width deviation
        let ideal_width = self.max_width * 0.8; // 80% of max width is ideal
        let width_penalty = ((line_width - ideal_width) / ideal_width).abs();

        // Add penalty for breaking after certain characters
        let char_penalty = if break_position > 0 {
            match text.chars().nth(break_position - 1) {
                Some('.') | Some('!') | Some('?') => 0.1, // Small penalty after sentence end
                Some(',') | Some(';') | Some(':') => 0.2, // Medium penalty after punctuation
                Some('(') | Some('[') | Some('{') => 0.5, // High penalty after opening brackets
                _ => 0.0,
            }
        } else {
            0.0
        };

        Ok(width_penalty + char_penalty)
    }

    /// Helper: Select breaks using width-based algorithm
    fn select_breaks_by_width(
        &self,
        break_positions: &[usize],
        run: &ShapedRun,
        base_offset: usize,
    ) -> Result<Vec<usize>, BidiError> {
        let mut selected = Vec::new();
        let mut current_width: f32 = 0.0;
        let mut last_break_glyph_idx = 0;
        let mut last_valid_break: Option<(usize, usize)> = None; // (text_position, glyph_index)

        for &break_pos in break_positions {
            // Calculate glyph index for this break position
            let glyph_idx = break_pos - base_offset;
            
            // Calculate width of segment from last break to this position
            let segment_width = self.calculate_segment_width(run, last_break_glyph_idx, glyph_idx)?;

            // Check if adding this segment would exceed max_width
            if current_width + segment_width > self.max_width {
                // Need to break at the LAST valid position, not current
                if let Some((valid_pos, valid_glyph_idx)) = last_valid_break {
                    selected.push(valid_pos);
                    // Reset from the break position
                    last_break_glyph_idx = valid_glyph_idx;
                    // Recalculate width from break position to current position
                    current_width = self.calculate_segment_width(run, last_break_glyph_idx, glyph_idx)?;
                } else {
                    // No valid break found, force break at current position
                    selected.push(break_pos);
                    current_width = 0.0;
                    last_break_glyph_idx = glyph_idx;
                }
            } else {
                // This segment fits, continue accumulating
                current_width += segment_width;
            }

            // Track this as a potential break point for next iteration
            last_valid_break = Some((break_pos, glyph_idx));
        }

        Ok(selected)
    }

    /// Helper: Create ShapedRun from VisualRun for analysis
    fn create_shaped_run_from_visual(
        &self,
        visual_run: &crate::bidi::types::VisualRun,
        font_system: &mut FontSystem,
        metrics: Metrics,
    ) -> Result<ShapedRun, BidiError> {
        // Convert Direction to TextDirection
        let direction = match visual_run.direction {
            crate::bidi::types::Direction::LeftToRight => TextDirection::LeftToRight,
            crate::bidi::types::Direction::RightToLeft => TextDirection::RightToLeft,
            crate::bidi::types::Direction::Auto => TextDirection::LeftToRight,
        };

        // Create a Buffer to properly shape the text and extract glyphs
        let mut temp_buffer = Buffer::new(font_system, metrics);
        temp_buffer.set_text(
            font_system,
            &visual_run.text,
            &Attrs::new(),
            Shaping::Advanced,
        );
        temp_buffer.set_size(font_system, Some(f32::INFINITY), None);

        // Extract glyphs from shaped layout runs
        let mut glyphs = Vec::new();
        let mut total_width: f32 = 0.0;
        let mut max_ascent: f32 = 0.0;
        let mut max_descent: f32 = 0.0;

        for layout_run in temp_buffer.layout_runs() {
            max_ascent = max_ascent.max(layout_run.line_height * 0.8);
            max_descent = max_descent.max(layout_run.line_height * 0.2);

            for glyph in layout_run.glyphs {
                glyphs.push(crate::shaping::types::ShapedGlyph {
                    glyph_id: glyph.glyph_id,
                    cluster: glyph.start as u32,
                    x_advance: glyph.w,
                    y_advance: 0.0,
                    x_offset: glyph.x,
                    y_offset: glyph.y,
                    flags: crate::shaping::types::GlyphFlags::empty(),
                    font_size: metrics.font_size,
                    color: None,
                });
                total_width += glyph.w;
            }
        }

        let level = unicode_bidi::Level::new(visual_run.level).map_err(|e| {
            BidiError::ProcessingFailed(format!("Invalid bidi level: {:?}", e))
        })?;

        Ok(ShapedRun {
            glyphs,
            script: visual_run.script.clone().into(),
            direction,
            language: None,
            level,
            width: total_width,
            height: max_ascent + max_descent,
            ascent: max_ascent,
            descent: max_descent,
            line_gap: metrics.line_height - max_ascent - max_descent,
            start_index: visual_run.start_index,
            end_index: visual_run.end_index,
        })
    }

    /// Helper: Calculate width of text segment
    fn calculate_segment_width(
        &self,
        run: &ShapedRun,
        start: usize,
        end: usize,
    ) -> Result<f32, BidiError> {
        let end_pos = end.min(run.glyphs.len());
        if start >= end_pos {
            return Ok(0.0);
        }
        let segment_glyphs = &run.glyphs[start..end_pos];
        Ok(segment_glyphs.iter().map(|g| g.x_advance).sum())
    }
}
