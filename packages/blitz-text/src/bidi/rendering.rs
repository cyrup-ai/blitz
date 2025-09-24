//! BiDi rendering engine
//!
//! This module handles the rendering of bidirectional text to various targets,
//! including glyph run rendering and line-by-line output.

use super::types::{BidiError, Direction, LineMetrics, ProcessedBidi, VisualRun};
use crate::types::{ShapedGlyph, ShapedRun};

/// Sophisticated cluster-to-glyph mapping for complex scripts
pub struct ClusterMapper {
    /// Maps logical character indices to glyph clusters
    char_to_cluster: Vec<u32>,
    /// Maps glyph clusters to visual glyph ranges
    cluster_to_glyphs: std::collections::HashMap<u32, std::ops::Range<usize>>,
}

impl ClusterMapper {
    /// Create cluster mapping from shaped run
    pub fn from_shaped_run(shaped_run: &ShapedRun, text_range: std::ops::Range<usize>) -> Self {
        let mut char_to_cluster = vec![0u32; text_range.len()];
        let mut cluster_to_glyphs = std::collections::HashMap::new();

        // Build character to cluster mapping
        for (glyph_idx, glyph) in shaped_run.glyphs.iter().enumerate() {
            let text_idx = glyph.cluster as usize;
            if text_idx >= text_range.start && text_idx < text_range.end {
                let local_idx = text_idx - text_range.start;
                char_to_cluster[local_idx] = glyph.cluster;

                // Update cluster to glyph range mapping
                cluster_to_glyphs
                    .entry(glyph.cluster)
                    .and_modify(|range: &mut std::ops::Range<usize>| {
                        range.start = range.start.min(glyph_idx);
                        range.end = range.end.max(glyph_idx + 1);
                    })
                    .or_insert(glyph_idx..glyph_idx + 1);
            }
        }

        Self {
            char_to_cluster,
            cluster_to_glyphs,
        }
    }

    /// Extract glyphs for text range with proper cluster handling
    pub fn extract_glyphs_for_range(
        &self,
        text_range: std::ops::Range<usize>,
        shaped_run: &ShapedRun,
    ) -> Vec<ShapedGlyph> {
        let mut result_glyphs = Vec::new();
        let mut covered_clusters = std::collections::HashSet::new();

        // Find all clusters that intersect with the text range
        for text_idx in text_range {
            if text_idx < self.char_to_cluster.len() {
                let cluster = self.char_to_cluster[text_idx];
                if !covered_clusters.contains(&cluster) {
                    covered_clusters.insert(cluster);

                    // Add all glyphs for this cluster
                    if let Some(glyph_range) = self.cluster_to_glyphs.get(&cluster) {
                        for glyph_idx in glyph_range.clone() {
                            if glyph_idx < shaped_run.glyphs.len() {
                                result_glyphs.push(shaped_run.glyphs[glyph_idx].clone());
                            }
                        }
                    }
                }
            }
        }

        // Sort glyphs by their original order in the shaped run
        result_glyphs.sort_by_key(|glyph| {
            shaped_run
                .glyphs
                .iter()
                .position(|g| g.glyph_id == glyph.glyph_id)
                .unwrap_or(usize::MAX)
        });

        result_glyphs
    }

    /// Handle complex script ligatures and decompositions
    pub fn handle_complex_clusters(&self, cluster: u32, shaped_run: &ShapedRun) -> ClusterInfo {
        if let Some(glyph_range) = self.cluster_to_glyphs.get(&cluster) {
            let glyphs: Vec<_> = glyph_range
                .clone()
                .map(|idx| &shaped_run.glyphs[idx])
                .collect();

            ClusterInfo {
                cluster_id: cluster,
                glyph_count: glyphs.len(),
                is_ligature: glyphs.len() > 1 && glyphs.iter().all(|g| g.cluster == cluster),
                is_decomposition: glyphs.len() == 1
                    && self
                        .char_to_cluster
                        .iter()
                        .filter(|&&c| c == cluster)
                        .count()
                        > 1,
                total_advance: glyphs.iter().map(|g| g.x_advance).sum(),
            }
        } else {
            ClusterInfo::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct ClusterInfo {
    pub cluster_id: u32,
    pub glyph_count: usize,
    pub is_ligature: bool,
    pub is_decomposition: bool,
    pub total_advance: f32,
}

/// Trait for BiDi render targets
pub trait BidiRenderTarget {
    /// Render a run of glyphs with directional information
    fn render_glyph_run(
        &mut self,
        glyphs: &[ShapedGlyph],
        x_offset: f32,
        y_offset: f32,
        direction: Direction,
        level: u8,
    ) -> Result<(), BidiError>;

    /// Begin a new line in the render target
    fn begin_line(&mut self, line_index: usize, y_offset: f32) -> Result<(), BidiError>;

    /// End the current line in the render target  
    fn end_line(&mut self, line_index: usize) -> Result<(), BidiError>;
}

/// BiDi rendering engine
pub struct BidiRenderer {
    default_direction: Direction,
}

impl BidiRenderer {
    /// Create new BiDi renderer
    pub fn new(default_direction: Direction) -> Self {
        Self { default_direction }
    }

    /// Set default text direction
    pub fn set_default_direction(&mut self, direction: Direction) {
        self.default_direction = direction;
    }

    /// Render bidirectional text to target (legacy single-line version)
    pub fn render_bidi_text(
        &self,
        target: &mut dyn BidiRenderTarget,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        y_offset: f32,
    ) -> Result<(), BidiError> {
        self.render_bidi_text_with_line_index(target, processed_bidi, shaped_runs, 0, y_offset)
    }

    /// Render bidirectional text to target with specified line index
    pub fn render_bidi_text_with_line_index(
        &self,
        target: &mut dyn BidiRenderTarget,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        line_index: usize,
        y_offset: f32,
    ) -> Result<(), BidiError> {
        target.begin_line(line_index, y_offset)?;

        let mut current_x = 0.0;

        // Render visual runs in visual order
        for visual_run in &processed_bidi.visual_runs {
            // Find corresponding shaped run
            if let Some(shaped_run) = self.find_shaped_run_for_visual(visual_run, shaped_runs) {
                // Extract glyphs for this visual run
                let run_glyphs = self.extract_glyphs_for_run(visual_run, shaped_run)?;

                // Render the glyph run
                target.render_glyph_run(
                    &run_glyphs,
                    current_x,
                    y_offset,
                    visual_run.direction,
                    visual_run.level,
                )?;

                // Advance x position
                current_x += self.calculate_run_width(&run_glyphs);
            }
        }

        target.end_line(line_index)?;
        Ok(())
    }

    /// Render multi-line bidirectional text to target
    pub fn render_multiline_bidi_text(
        &self,
        target: &mut dyn BidiRenderTarget,
        lines: &[(ProcessedBidi, Vec<ShapedRun>)],
        line_height: f32,
    ) -> Result<(), BidiError> {
        for (line_index, (processed_bidi, shaped_runs)) in lines.iter().enumerate() {
            let y_offset = line_index as f32 * line_height;
            self.render_bidi_text_with_line_index(
                target,
                processed_bidi,
                shaped_runs,
                line_index,
                y_offset,
            )?;
        }
        Ok(())
    }

    /// Render a single line of bidirectional text
    fn render_single_line(
        &self,
        target: &mut dyn BidiRenderTarget,
        processed_bidi: &ProcessedBidi,
        shaped_runs: &[ShapedRun],
        line_index: usize,
        y_offset: f32,
        line_metrics: &LineMetrics,
    ) -> Result<f32, BidiError> {
        target.begin_line(line_index, y_offset)?;

        let mut current_x = 0.0;
        let mut line_width = 0.0;

        // Process visual runs in their visual order
        for visual_run in &processed_bidi.visual_runs {
            if let Some(shaped_run) = self.find_shaped_run_for_visual(visual_run, shaped_runs) {
                let run_glyphs = self.extract_glyphs_for_run(visual_run, shaped_run)?;

                // Calculate run width
                let run_width = self.calculate_run_width(&run_glyphs);

                // Render the run
                target.render_glyph_run(
                    &run_glyphs,
                    current_x,
                    y_offset + line_metrics.baseline_offset,
                    visual_run.direction,
                    visual_run.level,
                )?;

                current_x += run_width;
                line_width += run_width;
            }
        }

        target.end_line(line_index)?;
        Ok(line_width)
    }

    /// Find shaped run corresponding to visual run
    fn find_shaped_run_for_visual<'a>(
        &self,
        visual_run: &VisualRun,
        shaped_runs: &'a [ShapedRun],
    ) -> Option<&'a ShapedRun> {
        // Find shaped run that overlaps with this visual run
        for shaped_run in shaped_runs {
            // Check if ranges overlap
            let shaped_start = shaped_run.start_index;
            let shaped_end = shaped_run.end_index;

            if visual_run.start_index < shaped_end && visual_run.end_index > shaped_start {
                return Some(shaped_run);
            }
        }
        None
    }

    /// Extract glyphs for a specific visual run from a shaped run
    fn extract_glyphs_for_run(
        &self,
        visual_run: &VisualRun,
        shaped_run: &ShapedRun,
    ) -> Result<Vec<ShapedGlyph>, BidiError> {
        let mut run_glyphs = Vec::new();

        // Calculate the overlap between visual run and shaped run
        let visual_start = visual_run.start_index;
        let visual_end = visual_run.end_index;
        let shaped_start = shaped_run.start_index;
        let shaped_end = shaped_run.end_index;

        let overlap_start = visual_start.max(shaped_start);
        let overlap_end = visual_end.min(shaped_end);

        if overlap_start >= overlap_end {
            return Ok(run_glyphs); // No overlap
        }

        // Extract glyphs using sophisticated cluster mapping
        let text_range = overlap_start..overlap_end;
        let cluster_mapper = ClusterMapper::from_shaped_run(shaped_run, shaped_start..shaped_end);
        run_glyphs = cluster_mapper.extract_glyphs_for_range(text_range, shaped_run);

        // If this is an RTL run, reverse the glyphs
        if visual_run.direction == Direction::RightToLeft {
            run_glyphs.reverse();
        }

        Ok(run_glyphs)
    }

    /// Calculate the total width of a glyph run
    fn calculate_run_width(&self, glyphs: &[ShapedGlyph]) -> f32 {
        glyphs.iter().map(|glyph| glyph.x_advance).sum()
    }

    /// Calculate line metrics for rendering
    pub fn calculate_line_metrics(
        &self,
        shaped_runs: &[ShapedRun],
        line_height: f32,
    ) -> LineMetrics {
        let mut max_ascent: f32 = 0.0;
        let mut max_descent: f32 = 0.0;

        for shaped_run in shaped_runs {
            max_ascent = max_ascent.max(shaped_run.ascent as f32);
            max_descent = max_descent.max(shaped_run.descent as f32);
        }

        let total_height = max_ascent + max_descent;
        let baseline_offset = max_ascent;

        LineMetrics {
            line_height: line_height.max(total_height),
            baseline_offset,
            ascent: max_ascent,
            descent: max_descent,
        }
    }

    /// Render text with automatic line wrapping
    pub fn render_wrapped_text(
        &self,
        target: &mut dyn BidiRenderTarget,
        lines: &[(ProcessedBidi, Vec<ShapedRun>)],
        max_width: f32,
        line_height: f32,
    ) -> Result<f32, BidiError> {
        let mut total_height = 0.0;

        for (line_index, (processed_bidi, shaped_runs)) in lines.iter().enumerate() {
            let y_offset = total_height;
            let line_metrics = self.calculate_line_metrics(shaped_runs, line_height);

            let line_width = self.render_single_line(
                target,
                processed_bidi,
                shaped_runs,
                line_index,
                y_offset,
                &line_metrics,
            )?;

            total_height += line_metrics.line_height;

            // Check if line exceeds max width (for debugging/validation)
            if line_width > max_width {
                // In a production system, this might trigger rewrapping
                eprintln!(
                    "Warning: Line {} width ({}) exceeds max width ({})",
                    line_index, line_width, max_width
                );
            }
        }

        Ok(total_height)
    }

    /// Get rendering statistics
    pub fn get_rendering_stats(&self) -> RenderingStats {
        RenderingStats {
            lines_rendered: 0, // Would be tracked in a real implementation
            glyphs_rendered: 0,
            total_width: 0.0,
            total_height: 0.0,
        }
    }
}

/// Rendering statistics
#[derive(Debug, Clone)]
pub struct RenderingStats {
    pub lines_rendered: usize,
    pub glyphs_rendered: usize,
    pub total_width: f32,
    pub total_height: f32,
}

/// Simple render target for testing
pub struct TestRenderTarget {
    pub rendered_runs: Vec<RenderedRun>,
    pub current_line: usize,
}

/// Information about a rendered run
#[derive(Debug, Clone)]
pub struct RenderedRun {
    pub line_index: usize,
    pub x_offset: f32,
    pub y_offset: f32,
    pub direction: Direction,
    pub level: u8,
    pub glyph_count: usize,
    pub width: f32,
}

impl TestRenderTarget {
    /// Create new test render target
    pub fn new() -> Self {
        Self {
            rendered_runs: Vec::new(),
            current_line: 0,
        }
    }

    /// Get total rendered width
    pub fn total_width(&self) -> f32 {
        self.rendered_runs
            .iter()
            .map(|run| run.x_offset + run.width)
            .fold(0.0, f32::max)
    }

    /// Get total rendered height
    pub fn total_height(&self) -> f32 {
        self.rendered_runs
            .iter()
            .map(|run| run.y_offset)
            .fold(0.0, f32::max)
    }
}

impl BidiRenderTarget for TestRenderTarget {
    fn render_glyph_run(
        &mut self,
        glyphs: &[ShapedGlyph],
        x_offset: f32,
        y_offset: f32,
        direction: Direction,
        level: u8,
    ) -> Result<(), BidiError> {
        let width: f32 = glyphs.iter().map(|g| g.x_advance).sum();

        self.rendered_runs.push(RenderedRun {
            line_index: self.current_line,
            x_offset,
            y_offset,
            direction,
            level,
            glyph_count: glyphs.len(),
            width,
        });

        Ok(())
    }

    fn begin_line(&mut self, line_index: usize, _y_offset: f32) -> Result<(), BidiError> {
        self.current_line = line_index;
        Ok(())
    }

    fn end_line(&mut self, _line_index: usize) -> Result<(), BidiError> {
        Ok(())
    }
}
