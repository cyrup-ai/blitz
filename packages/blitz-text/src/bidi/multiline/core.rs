//! Core multi-line BiDi processor and configuration
//!
//! This module provides the main MultiLineBidiProcessor struct and core
//! processing functionality for multi-line bidirectional text.

use super::super::processing::BidiProcessor;
use super::super::types::{
    BidiError, BidiRenderOptions, Direction, LineBidi, LineBreakInfo, LineMetrics,
    MultiLineBidiResult, ParagraphBidi,
};
use super::line_breaking::LineBreaker;
use super::statistics::MultiLineStats;

/// Multi-line BiDi text processor
pub struct MultiLineBidiProcessor {
    processor: BidiProcessor,
    max_width: f32,
    line_height: f32,
    line_breaker: LineBreaker,
}

impl MultiLineBidiProcessor {
    /// Create new multi-line processor
    pub fn new(default_direction: Direction, max_width: f32, line_height: f32) -> Self {
        Self {
            processor: BidiProcessor::new(default_direction),
            max_width,
            line_height,
            line_breaker: LineBreaker::new(max_width),
        }
    }

    /// Set maximum line width
    pub fn set_max_width(&mut self, max_width: f32) {
        self.max_width = max_width;
        self.line_breaker.set_max_width(max_width);
    }

    /// Set line height
    pub fn set_line_height(&mut self, line_height: f32) {
        self.line_height = line_height;
    }

    /// Get current max width
    pub fn max_width(&self) -> f32 {
        self.max_width
    }

    /// Get current line height
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Process multi-line bidirectional text
    pub fn process_multiline_bidi_text(
        &self,
        text: &str,
        options: &BidiRenderOptions,
    ) -> Result<MultiLineBidiResult, BidiError> {
        // Split text into paragraphs
        let paragraphs = self.split_into_paragraphs(text);
        let mut processed_paragraphs = Vec::new();

        for (paragraph_index, paragraph_text) in paragraphs.iter().enumerate() {
            let paragraph_bidi =
                self.process_paragraph(paragraph_text, paragraph_index, options)?;
            processed_paragraphs.push(paragraph_bidi);
        }

        let total_lines = processed_paragraphs.iter().map(|p| p.lines.len()).sum();

        Ok(MultiLineBidiResult {
            paragraphs: processed_paragraphs,
            total_lines,
            base_direction: options.base_direction,
        })
    }

    /// Process a single paragraph with line breaking
    pub(super) fn process_paragraph(
        &self,
        text: &str,
        paragraph_index: usize,
        options: &BidiRenderOptions,
    ) -> Result<ParagraphBidi, BidiError> {
        // First, process the entire paragraph for BiDi
        let processed_bidi = self.processor.process_bidi_text(text, options)?;

        // Then break into lines based on width constraints
        let line_breaks = self
            .line_breaker
            .calculate_line_breaks(text, &processed_bidi)?;
        let mut lines = Vec::new();

        for (line_index, line_info) in line_breaks.iter().enumerate() {
            let line_bidi = self.process_line(&line_info.text, line_index, options, line_info)?;
            lines.push(line_bidi);
        }

        Ok(ParagraphBidi {
            paragraph_index,
            lines,
            base_direction: processed_bidi.base_direction,
        })
    }

    /// Process a single line of text
    pub(super) fn process_line(
        &self,
        line_text: &str,
        line_index: usize,
        options: &BidiRenderOptions,
        line_info: &LineBreakInfo,
    ) -> Result<LineBidi, BidiError> {
        let processed_bidi = self.processor.process_bidi_text(line_text, options)?;

        // Calculate line metrics
        let baseline_offset = self.line_height * 0.8; // 80% of line height
        let visual_width = line_info
            .line_widths
            .get(line_index)
            .copied()
            .unwrap_or(0.0);

        Ok(LineBidi {
            line_index,
            processed_bidi: (*processed_bidi).clone(),
            line_height: self.line_height,
            baseline_offset,
            visual_width,
            break_opportunity: line_info
                .break_opportunities
                .get(line_index)
                .copied()
                .unwrap_or(false),
        })
    }

    /// Split text into paragraphs
    pub(super) fn split_into_paragraphs<'a>(&self, text: &'a str) -> Vec<&'a str> {
        text.split('\n').collect()
    }

    /// Calculate total height of multi-line text
    pub fn calculate_total_height(&self, multiline_result: &MultiLineBidiResult) -> f32 {
        multiline_result.total_lines as f32 * self.line_height
    }

    /// Get line at specific y coordinate
    pub fn get_line_at_y(
        &self,
        multiline_result: &MultiLineBidiResult,
        y: f32,
    ) -> Option<(usize, usize)> {
        let line_index = (y / self.line_height).floor() as usize;
        let mut current_line = 0;

        for (paragraph_index, paragraph) in multiline_result.paragraphs.iter().enumerate() {
            if current_line + paragraph.lines.len() > line_index {
                let line_in_paragraph = line_index - current_line;
                return Some((paragraph_index, line_in_paragraph));
            }
            current_line += paragraph.lines.len();
        }

        None
    }

    /// Get line metrics for a specific line
    pub fn get_line_metrics(
        &self,
        multiline_result: &MultiLineBidiResult,
        paragraph_index: usize,
        line_index: usize,
    ) -> Option<LineMetrics> {
        let paragraph = multiline_result.paragraphs.get(paragraph_index)?;
        let line = paragraph.lines.get(line_index)?;

        Some(LineMetrics {
            line_height: line.line_height,
            baseline_offset: line.baseline_offset,
            ascent: line.baseline_offset,
            descent: line.line_height - line.baseline_offset,
        })
    }

    /// Get statistics about multi-line processing
    pub fn get_multiline_stats(&self, multiline_result: &MultiLineBidiResult) -> MultiLineStats {
        MultiLineStats::from_result(multiline_result, self.line_height)
    }

    /// Wrap text with line breaking and BiDi processing
    pub fn wrap_text(
        &self,
        text: &str,
        options: &BidiRenderOptions,
        shaped_runs: &[crate::types::ShapedRun],
    ) -> Result<
        Vec<(
            super::super::types::ProcessedBidi,
            Vec<crate::types::ShapedRun>,
        )>,
        BidiError,
    > {
        // Process multiline text
        let multiline_result = self.process_multiline_bidi_text(text, options)?;

        // Convert to expected return format
        let mut result = Vec::new();
        for paragraph in multiline_result.paragraphs {
            for line in paragraph.lines {
                // For each line, pair it with the shaped runs
                result.push((line.processed_bidi, shaped_runs.to_vec()));
            }
        }

        Ok(result)
    }
}
