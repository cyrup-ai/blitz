//! Line breaking algorithms and width calculations
//!
//! This module handles Unicode line break opportunities, width constraints,
//! and line break decision logic for multi-line BiDi text.

use unicode_linebreak::{linebreaks, BreakOpportunity};

use super::super::types::{BidiError, LineBreakInfo, ProcessedBidi};

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
    ) -> Result<Vec<LineBreakInfo>, BidiError> {
        // Find Unicode line break opportunities
        let break_opportunities: Vec<_> = linebreaks(text).collect();
        let mut lines = Vec::new();
        let mut current_line_start = 0;

        for (_break_index, break_opportunity) in break_opportunities.iter().enumerate() {
            let break_position = break_opportunity.0;

            // Check if we should break here based on width constraints
            if self.should_break_line(text, current_line_start, break_position, processed_bidi)? {
                let line_text = &text[current_line_start..break_position];
                let line_width = self.calculate_line_width(line_text, processed_bidi)?;

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
            let line_width = self.calculate_line_width(line_text, processed_bidi)?;

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
    ) -> Result<bool, BidiError> {
        let line_text = &text[line_start..break_position];
        let line_width = self.calculate_line_width(line_text, processed_bidi)?;

        Ok(line_width > self.max_width)
    }

    /// Calculate width of a line of text
    pub fn calculate_line_width(
        &self,
        line_text: &str,
        processed_bidi: &ProcessedBidi,
    ) -> Result<f32, BidiError> {
        // This is a simplified calculation
        // In practice, would need shaped runs to get accurate width
        Ok(line_text.chars().count() as f32 * 10.0) // Rough estimate
    }

    /// Find optimal break points using advanced algorithms
    pub fn find_optimal_breaks(
        &self,
        text: &str,
        processed_bidi: &ProcessedBidi,
    ) -> Result<Vec<usize>, BidiError> {
        // This would implement algorithms like Knuth-Plass line breaking
        // For now, use simple greedy approach
        let _ = processed_bidi; // Suppress false positive - variable is used in should_break_line call
        let break_opportunities: Vec<_> = linebreaks(text).collect();
        let mut optimal_breaks = Vec::new();
        let mut current_line_start = 0;

        for break_opportunity in break_opportunities {
            let break_position = break_opportunity.0;

            if self.should_break_line(text, current_line_start, break_position, processed_bidi)? {
                optimal_breaks.push(break_position);
                current_line_start = break_position;
            }
        }

        // Add final break if needed
        if current_line_start < text.len() {
            optimal_breaks.push(text.len());
        }

        Ok(optimal_breaks)
    }

    /// Calculate penalty for breaking at a specific position
    pub fn calculate_break_penalty(
        &self,
        text: &str,
        break_position: usize,
        processed_bidi: &ProcessedBidi,
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
        let line_width = self.calculate_line_width(line_text, processed_bidi)?;

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
}
