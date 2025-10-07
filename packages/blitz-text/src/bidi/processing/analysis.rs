//! BiDi script analysis and visual run processing
//!
//! This module handles script analysis, visual run creation, and paragraph processing.

use unicode_bidi::BidiInfo;
use unicode_script::{Script, UnicodeScript};

use super::super::types::{BidiError, Direction, SerializableScript, VisualRun};
use crate::shaping::types::ScriptComplexity;

/// BiDi text analysis utilities
pub struct BidiAnalyzer;

impl BidiAnalyzer {
    /// Create new BiDi analyzer
    pub fn new() -> Self {
        Self
    }

    /// Create visual runs from BiDi processing results
    pub fn create_visual_runs(
        &self,
        text: &str,
        bidi_info: &BidiInfo,
        paragraph: &unicode_bidi::ParagraphInfo,
    ) -> Result<Vec<VisualRun>, BidiError> {
        let mut visual_runs = Vec::new();
        let line_range = paragraph.range.clone();

        // Use unicode-bidi's visual_runs API to get proper reordering
        let (levels, level_runs) = bidi_info.visual_runs(paragraph, line_range.clone());

        // Convert unicode-bidi LevelRuns to our VisualRun type
        for (visual_index, level_run) in level_runs.iter().enumerate() {
            // level_run is a Range<usize> representing byte indices
            let byte_start = level_run.start;
            let byte_end = level_run.end;

            // Get text for this run
            let run_text = &text[byte_start..byte_end];

            // Convert byte indices to character indices
            let char_start = text[..byte_start].chars().count();
            let char_end = char_start + run_text.chars().count();

            // Get level for this run
            let level_value = if byte_start < levels.len() {
                levels[byte_start].number()
            } else {
                0
            };

            // Determine direction from level (odd = RTL, even = LTR)
            let direction = if level_value % 2 == 1 {
                Direction::RightToLeft
            } else {
                Direction::LeftToRight
            };

            // Analyze script and complexity
            let (script, complexity) = self.analyze_run_script(run_text);

            visual_runs.push(VisualRun {
                text: run_text.to_string(),
                start_index: char_start,
                end_index: char_end,
                direction,
                level: level_value,
                script: SerializableScript::from_script(script),
                complexity,
                visual_order: visual_index,
            });
        }

        Ok(visual_runs)
    }

    /// Analyze script and complexity for a text run
    pub fn analyze_run_script(&self, text: &str) -> (Script, ScriptComplexity) {
        let mut scripts = std::collections::HashSet::new();
        let mut has_complex = false;

        for ch in text.chars() {
            let script = ch.script();
            scripts.insert(script);

            // Check for complex script characteristics
            match script {
                Script::Arabic | Script::Hebrew => has_complex = true,
                Script::Devanagari
                | Script::Bengali
                | Script::Gujarati
                | Script::Gurmukhi
                | Script::Kannada
                | Script::Malayalam
                | Script::Oriya
                | Script::Tamil
                | Script::Telugu => has_complex = true,
                Script::Thai | Script::Lao | Script::Myanmar => has_complex = true,
                _ => {}
            }
        }

        let primary_script = scripts.iter().next().copied().unwrap_or(Script::Latin);
        let complexity = if has_complex || scripts.len() > 1 {
            ScriptComplexity::Complex
        } else {
            ScriptComplexity::Simple
        };

        (primary_script, complexity)
    }

    /// Process all paragraphs in multi-paragraph text with zero-allocation optimization
    pub fn process_all_paragraphs(
        &self,
        text: &str,
        bidi_info: &BidiInfo,
    ) -> Result<(Vec<VisualRun>, Vec<usize>, Vec<usize>), BidiError> {
        let mut all_visual_runs = Vec::with_capacity(bidi_info.paragraphs.len() * 4);
        let char_count = text.chars().count();
        let mut logical_to_visual = vec![0; char_count];
        let mut visual_to_logical = vec![0; char_count];

        let mut visual_char_offset = 0;
        let mut current_visual_run_index = 0;

        // Process each paragraph independently
        for paragraph in &bidi_info.paragraphs {
            // Create visual runs for this paragraph
            let paragraph_visual_runs = self.create_visual_runs(text, bidi_info, paragraph)?;

            // Create index mappings for this paragraph
            let (para_logical_to_visual, _para_visual_to_logical) =
                self.create_index_mappings(text, bidi_info, paragraph)?;

            // Merge paragraph runs into global visual runs with offset correction
            for mut visual_run in paragraph_visual_runs {
                visual_run.visual_order = current_visual_run_index;
                all_visual_runs.push(visual_run);
                current_visual_run_index += 1;
            }

            // Merge paragraph index mappings with global offset correction
            let paragraph_char_start = 0; // For single paragraph, start at 0
            let paragraph_char_count = text.chars().count();

            for i in 0..paragraph_char_count {
                let logical_idx = paragraph_char_start + i;
                let visual_idx = visual_char_offset + para_logical_to_visual[i];

                logical_to_visual[logical_idx] = visual_idx;
                visual_to_logical[visual_idx] = logical_idx;
            }

            visual_char_offset += paragraph_char_count;
        }

        Ok((all_visual_runs, logical_to_visual, visual_to_logical))
    }

    /// Create logical-to-visual and visual-to-logical index mappings
    pub fn create_index_mappings(
        &self,
        text: &str,
        bidi_info: &BidiInfo,
        paragraph: &unicode_bidi::ParagraphInfo,
    ) -> Result<(Vec<usize>, Vec<usize>), BidiError> {
        let line_range = paragraph.range.clone();

        // Get reordered levels using unicode-bidi API
        let reordered_levels = bidi_info.reordered_levels(paragraph, line_range.clone());

        // Use unicode-bidi's reorder_visual to create the index mapping
        // This returns Vec<usize> where indexMap[visualIndex] == logicalIndex
        let visual_to_logical = BidiInfo::reorder_visual(&reordered_levels);

        // Create inverse mapping (logical to visual)
        let char_count = text.chars().count();
        let mut logical_to_visual = vec![0; char_count];

        for (visual_idx, &logical_idx) in visual_to_logical.iter().enumerate() {
            if logical_idx < logical_to_visual.len() {
                logical_to_visual[logical_idx] = visual_idx;
            }
        }

        Ok((logical_to_visual, visual_to_logical))
    }
}

impl Default for BidiAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
