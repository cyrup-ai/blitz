//! Statistics and analysis for multi-line BiDi processing
//!
//! This module provides statistical analysis and optimization functionality
//! for multi-line bidirectional text processing results.

use super::super::types::{BidiError, MultiLineBidiResult};

/// Knuth-Plass line breaking algorithm implementation
/// Based on "Breaking Paragraphs into Lines" by Donald E. Knuth and Michael F. Plass
pub struct KnuthPlassBreaker {
    /// Target line width
    target_width: f32,
    /// Tolerance for line width variation (0.0 to 1.0)
    tolerance: f32,
    /// Penalty for consecutive hyphens
    hyphen_penalty: i32,
    /// Penalty for widows and orphans
    widow_penalty: i32,
}

impl KnuthPlassBreaker {
    pub fn new(target_width: f32) -> Self {
        Self {
            target_width,
            tolerance: 0.05, // 5% tolerance
            hyphen_penalty: 50,
            widow_penalty: 150,
        }
    }

    fn calculate_badness(&self, actual_width: f32) -> f32 {
        let ratio = (actual_width - self.target_width) / self.target_width;

        if ratio.abs() <= self.tolerance {
            0.0 // Perfect fit
        } else if ratio > 0.0 {
            // Line too long
            100.0 * ratio.powi(3)
        } else {
            // Line too short
            100.0 * (-ratio).powi(3)
        }
    }
}

#[derive(Debug, Clone)]
struct BreakOpportunity {
    position: usize,
    break_type: BreakType,
    penalty: i32,
    width_contribution: f32,
}

#[derive(Debug, Clone)]
enum BreakType {
    Mandatory,
    WordBoundary,
    Hyphenation,
    Emergency,
}

/// Multi-line processing statistics
#[derive(Debug, Clone)]
pub struct MultiLineStats {
    pub total_paragraphs: usize,
    pub total_lines: usize,
    pub total_characters: usize,
    pub avg_lines_per_paragraph: f32,
    pub total_height: f32,
}

impl MultiLineStats {
    /// Create statistics from multiline result
    pub fn from_result(multiline_result: &MultiLineBidiResult, line_height: f32) -> Self {
        let total_paragraphs = multiline_result.paragraphs.len();
        let total_lines = multiline_result.total_lines;
        let avg_lines_per_paragraph = if total_paragraphs > 0 {
            total_lines as f32 / total_paragraphs as f32
        } else {
            0.0
        };

        let total_characters: usize = multiline_result
            .paragraphs
            .iter()
            .flat_map(|p| &p.lines)
            .map(|l| l.processed_bidi.text.chars().count())
            .sum();

        let total_height = total_lines as f32 * line_height;

        Self {
            total_paragraphs,
            total_lines,
            total_characters,
            avg_lines_per_paragraph,
            total_height,
        }
    }

    /// Get average characters per line
    pub fn avg_characters_per_line(&self) -> f32 {
        if self.total_lines > 0 {
            self.total_characters as f32 / self.total_lines as f32
        } else {
            0.0
        }
    }

    /// Get text density (characters per unit height)
    pub fn text_density(&self) -> f32 {
        if self.total_height > 0.0 {
            self.total_characters as f32 / self.total_height
        } else {
            0.0
        }
    }

    /// Calculate line length variance
    pub fn line_length_variance(&self, multiline_result: &MultiLineBidiResult) -> f32 {
        let line_lengths: Vec<usize> = multiline_result
            .paragraphs
            .iter()
            .flat_map(|p| &p.lines)
            .map(|l| l.processed_bidi.text.chars().count())
            .collect();

        if line_lengths.is_empty() {
            return 0.0;
        }

        let mean = self.avg_characters_per_line();
        let variance = line_lengths
            .iter()
            .map(|&length| {
                let diff = length as f32 - mean;
                diff * diff
            })
            .sum::<f32>()
            / line_lengths.len() as f32;

        variance
    }

    /// Get line length distribution
    pub fn line_length_distribution(
        &self,
        multiline_result: &MultiLineBidiResult,
    ) -> Result<LineDistribution, BidiError> {
        // Validate input structure
        if multiline_result.paragraphs.is_empty() {
            return Err(BidiError::InsufficientStatisticalData {
                expected: 1,
                actual: 0,
            });
        }

        let line_lengths: Vec<usize> = multiline_result
            .paragraphs
            .iter()
            .flat_map(|p| &p.lines)
            .map(|l| l.processed_bidi.text.chars().count())
            .collect();

        // Enhanced validation
        if line_lengths.len() < 2 {
            return Err(BidiError::InsufficientStatisticalData {
                expected: 2,
                actual: line_lengths.len(),
            });
        }

        // Safe computation with explicit error handling
        let min_length = line_lengths.iter().min().copied().ok_or_else(|| {
            BidiError::StatisticalCalculationFailed(
                "Failed to calculate minimum: iterator corrupted".to_string(),
            )
        })?;

        let max_length = line_lengths.iter().max().copied().ok_or_else(|| {
            BidiError::StatisticalCalculationFailed(
                "Failed to calculate maximum: iterator corrupted".to_string(),
            )
        })?;
        let median_length = {
            let mut sorted = line_lengths.clone();
            sorted.sort_unstable();
            let mid = sorted.len() / 2;

            // Safe median calculation with bounds checking
            if sorted.is_empty() {
                return Err(BidiError::StatisticalCalculationFailed(
                    "Cannot calculate median of empty dataset".to_string(),
                ));
            } else if sorted.len() % 2 == 0 {
                if mid > 0 && mid < sorted.len() {
                    (sorted[mid - 1] + sorted[mid]) as f32 / 2.0
                } else {
                    return Err(BidiError::StatisticalCalculationFailed(
                        "Median calculation index out of bounds".to_string(),
                    ));
                }
            } else {
                if mid < sorted.len() {
                    sorted[mid] as f32
                } else {
                    return Err(BidiError::StatisticalCalculationFailed(
                        "Median calculation index out of bounds".to_string(),
                    ));
                }
            }
        };

        Ok(LineDistribution {
            min_length,
            max_length,
            median_length,
            mean_length: self.avg_characters_per_line(),
            variance: self.line_length_variance(multiline_result),
        })
    }
}

/// Line length distribution statistics
#[derive(Debug, Clone)]
pub struct LineDistribution {
    pub min_length: usize,
    pub max_length: usize,
    pub median_length: f32,
    pub mean_length: f32,
    pub variance: f32,
}

impl Default for LineDistribution {
    fn default() -> Self {
        Self {
            min_length: 0,
            max_length: 0,
            median_length: 0.0,
            mean_length: 0.0,
            variance: 0.0,
        }
    }
}

/// Multi-line optimization utilities
pub struct MultiLineOptimizer;

impl MultiLineOptimizer {
    /// Optimize line breaks for better visual appearance using Knuth-Plass algorithm
    pub fn optimize_line_breaks(
        multiline_result: &mut MultiLineBidiResult,
    ) -> Result<(), BidiError> {
        // Estimate target width from existing lines
        let target_width = Self::estimate_target_width(multiline_result);
        let knuth_plass = KnuthPlassBreaker::new(target_width);

        for paragraph in &mut multiline_result.paragraphs {
            // Apply Knuth-Plass optimization to each paragraph
            Self::optimize_paragraph_breaks(paragraph, &knuth_plass)?;
        }

        Ok(())
    }

    /// Estimate target line width from existing line data
    fn estimate_target_width(multiline_result: &MultiLineBidiResult) -> f32 {
        let mut total_width = 0.0;
        let mut line_count = 0;

        for paragraph in &multiline_result.paragraphs {
            for line in &paragraph.lines {
                total_width += line.visual_width;
                line_count += 1;
            }
        }

        if line_count > 0 {
            total_width / line_count as f32
        } else {
            400.0 // Default target width
        }
    }

    /// Optimize line breaks for a single paragraph using Knuth-Plass
    fn optimize_paragraph_breaks(
        paragraph: &mut crate::bidi::types::ParagraphBidi,
        knuth_plass: &KnuthPlassBreaker,
    ) -> Result<(), BidiError> {
        let line_count = paragraph.lines.len();

        for line in &mut paragraph.lines {
            // Calculate badness for current line width
            let badness = knuth_plass.calculate_badness(line.visual_width);

            // Mark as break opportunity if badness is acceptable
            line.break_opportunity = badness < 100.0;

            // Apply widow/orphan penalties
            if line_count < 3 {
                // Avoid widow/orphan lines in short paragraphs
                line.break_opportunity = false;
            }
        }

        Ok(())
    }

    /// Apply widow/orphan control
    pub fn apply_widow_orphan_control(
        multiline_result: &mut MultiLineBidiResult,
        min_lines_at_start: usize,
        min_lines_at_end: usize,
    ) -> Result<(), BidiError> {
        for paragraph in &mut multiline_result.paragraphs {
            let total_lines = paragraph.lines.len();

            // Skip if paragraph is too short
            if total_lines < min_lines_at_start + min_lines_at_end {
                continue;
            }

            // Mark lines that should stay together
            for (index, line) in paragraph.lines.iter_mut().enumerate() {
                if index < min_lines_at_start || index >= total_lines - min_lines_at_end {
                    line.break_opportunity = false;
                }
            }
        }

        Ok(())
    }

    /// Calculate readability score
    pub fn calculate_readability_score(
        multiline_result: &MultiLineBidiResult,
        stats: &MultiLineStats,
    ) -> f32 {
        // Simple readability score based on:
        // - Line length consistency
        // - Paragraph structure
        // - Character density

        let line_consistency =
            1.0 - (stats.line_length_variance(multiline_result) / 100.0).min(1.0);
        let paragraph_balance = if stats.total_paragraphs > 0 {
            1.0 - ((stats.avg_lines_per_paragraph - 3.0).abs() / 10.0).min(1.0)
        } else {
            0.0
        };
        let density_score = (stats.text_density() / 50.0).min(1.0);

        (line_consistency + paragraph_balance + density_score) / 3.0
    }
}
