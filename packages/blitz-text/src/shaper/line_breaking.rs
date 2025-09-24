//! UAX #14 compliant line breaking with Unicode property tables

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::ShapingError;
use crate::line_breaking::{BreakClass, BreakOpportunity, LineBreakAnalyzer};
use crate::shaping::types::ShapedRun;

/// Statistics for line breaking operations
static LINE_BREAK_OPERATIONS: AtomicUsize = AtomicUsize::new(0);
static BREAK_OPPORTUNITIES_FOUND: AtomicUsize = AtomicUsize::new(0);

/// UAX #14 compliant line breaker
pub struct LineBreaker {
    analyzer: LineBreakAnalyzer,
}

impl LineBreaker {
    /// Create new line breaker
    pub fn new() -> Self {
        Self {
            analyzer: LineBreakAnalyzer::new(),
        }
    }

    /// UAX #14 compliant line breaking with Unicode property tables
    pub fn apply_line_breaking_optimized(
        &mut self,
        runs: Vec<ShapedRun>,
        max_width: f32,
    ) -> Result<Vec<ShapedRun>, ShapingError> {
        LINE_BREAK_OPERATIONS.fetch_add(1, Ordering::Relaxed);

        if runs.is_empty() {
            return Ok(runs);
        }

        let mut broken_runs = Vec::with_capacity(runs.len() * 2);

        for run in runs {
            let break_opportunities = self.analyzer.find_break_opportunities(&run)?;
            BREAK_OPPORTUNITIES_FOUND.fetch_add(break_opportunities.len(), Ordering::Relaxed);

            let line_segments = self.split_run_at_breaks(run, break_opportunities, max_width)?;
            broken_runs.extend(line_segments);
        }

        Ok(broken_runs)
    }

    /// Split a shaped run at valid break opportunities while preserving grapheme clusters
    fn split_run_at_breaks(
        &self,
        run: ShapedRun,
        break_opportunities: Vec<BreakOpportunity>,
        max_width: f32,
    ) -> Result<Vec<ShapedRun>, ShapingError> {
        if break_opportunities.is_empty() || run.width <= max_width {
            return Ok(vec![run]);
        }

        let mut segments = Vec::with_capacity(4);
        let mut current_width = 0.0;
        let mut segment_start = 0;
        let mut last_valid_break = 0;

        // Accumulate glyphs until we exceed max_width, then break at last valid opportunity
        for (glyph_idx, glyph) in run.glyphs.iter().enumerate() {
            current_width += glyph.x_advance;

            // Check if this position has a break opportunity
            if let Some(break_opp) = break_opportunities
                .iter()
                .find(|b| b.position == glyph.cluster as usize)
            {
                if break_opp.break_class == BreakClass::Allowed
                    || break_opp.break_class == BreakClass::Mandatory
                {
                    last_valid_break = glyph_idx;
                }
            }

            // If we exceed width and have a valid break point, create segment
            if current_width > max_width && last_valid_break > segment_start {
                let segment = self.create_run_segment(&run, segment_start, last_valid_break)?;
                segments.push(segment);

                segment_start = last_valid_break;
                current_width = run.glyphs[segment_start..=glyph_idx]
                    .iter()
                    .map(|g| g.x_advance)
                    .sum();
            }
        }

        // Add remaining glyphs as final segment
        if segment_start < run.glyphs.len() {
            let segment = self.create_run_segment(&run, segment_start, run.glyphs.len())?;
            segments.push(segment);
        }

        Ok(if segments.is_empty() {
            vec![run]
        } else {
            segments
        })
    }

    /// Create a run segment from glyph range while preserving metrics
    fn create_run_segment(
        &self,
        original_run: &ShapedRun,
        start_idx: usize,
        end_idx: usize,
    ) -> Result<ShapedRun, ShapingError> {
        if start_idx >= end_idx || end_idx > original_run.glyphs.len() {
            return Err(ShapingError::InvalidRange {
                start: start_idx,
                end: end_idx,
                length: original_run.glyphs.len(),
            });
        }

        let segment_glyphs = original_run.glyphs[start_idx..end_idx].to_vec();
        let segment_width: f32 = segment_glyphs.iter().map(|g| g.x_advance).sum();

        // Adjust glyph positions for segment start
        let mut adjusted_glyphs = segment_glyphs;
        if let Some(first_glyph) = adjusted_glyphs.first() {
            let x_offset = first_glyph.x_offset;
            for glyph in &mut adjusted_glyphs {
                glyph.x_offset -= x_offset;
            }
        }

        Ok(ShapedRun {
            glyphs: adjusted_glyphs,
            script: original_run.script,
            direction: original_run.direction,
            language: original_run.language.clone(),
            level: original_run.level,
            width: segment_width,
            height: original_run.height,
            ascent: original_run.ascent,
            descent: original_run.descent,
            line_gap: original_run.line_gap,
            start_index: original_run.start_index + start_idx,
            end_index: original_run.start_index + end_idx,
        })
    }

    /// Check if line breaking is needed for given width
    #[inline]
    pub fn needs_line_breaking(runs: &[ShapedRun], max_width: f32) -> bool {
        let total_width: f32 = runs.iter().map(|r| r.width).sum();
        total_width > max_width
    }

    /// Find optimal break points for text wrapping
    pub fn find_optimal_breaks(
        &self,
        runs: &[ShapedRun],
        max_width: f32,
    ) -> Result<Vec<usize>, ShapingError> {
        let mut break_points = Vec::new();
        let mut current_width = 0.0;
        let mut current_run_idx = 0;

        for (run_idx, run) in runs.iter().enumerate() {
            current_width += run.width;

            if current_width > max_width && run_idx > current_run_idx {
                // Add break point at start of current run
                break_points.push(run_idx);
                current_width = run.width;
                current_run_idx = run_idx;
            }
        }

        Ok(break_points)
    }

    /// Get line breaking statistics
    pub fn stats() -> LineBreakStats {
        LineBreakStats {
            operations: LINE_BREAK_OPERATIONS.load(Ordering::Relaxed),
            opportunities_found: BREAK_OPPORTUNITIES_FOUND.load(Ordering::Relaxed),
        }
    }

    /// Clear line breaking caches
    pub fn clear_caches(&mut self) {
        // Delegate to analyzer if it has caches
        // self.analyzer.clear_caches();
    }
}

impl Default for LineBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Line breaking statistics
#[derive(Debug, Clone)]
pub struct LineBreakStats {
    pub operations: usize,
    pub opportunities_found: usize,
}

impl LineBreakStats {
    /// Calculate average opportunities per operation
    #[inline]
    pub fn avg_opportunities_per_operation(&self) -> f64 {
        if self.operations > 0 {
            self.opportunities_found as f64 / self.operations as f64
        } else {
            0.0
        }
    }
}
