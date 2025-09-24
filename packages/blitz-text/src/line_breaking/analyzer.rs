//! Line break analyzer and opportunity optimization
//!
//! This module contains the main LineBreakAnalyzer struct and methods
//! for finding and optimizing break opportunities in text.

use super::character_classification::{classify_characters_simd, populate_ascii_properties};
use super::rule_application::{
    apply_pair_table_rules, calculate_break_penalty, calculate_break_priority,
};
use super::types::{BreakContextState, BreakOpportunity, LineBreakClass};
use crate::error::ShapingError;
use crate::shaping::types::ShapedRun;

/// UAX #14 compliant line break analyzer with precomputed property tables
pub struct LineBreakAnalyzer {
    /// Precomputed line break properties for fast character classification
    break_property_cache: [LineBreakClass; 256], // ASCII fast path
    /// SIMD-optimized character classification buffer
    classification_buffer: [LineBreakClass; 64], // Cache-aligned
    /// Context state for rule application
    context_state: BreakContextState,
}

impl LineBreakAnalyzer {
    /// Create new analyzer with precomputed property tables
    pub fn new() -> Self {
        let mut break_property_cache = [LineBreakClass::XX; 256];

        // Precompute ASCII character line break properties
        populate_ascii_properties(&mut break_property_cache);

        Self {
            break_property_cache,
            classification_buffer: [LineBreakClass::XX; 64],
            context_state: BreakContextState::new(),
        }
    }

    /// Find all break opportunities in a shaped run using UAX #14 algorithm
    pub fn find_break_opportunities(
        &mut self,
        run: &ShapedRun,
    ) -> Result<Vec<BreakOpportunity>, ShapingError> {
        let mut opportunities = Vec::with_capacity(run.glyphs.len() / 4);
        self.context_state.reset();

        // Extract text from glyphs to analyze character properties
        let text_chars = self.extract_characters_from_run(run)?;

        // SIMD-accelerated character classification where possible
        let char_classes = classify_characters_simd(&text_chars, &self.break_property_cache)?;

        // Apply UAX #14 rules with context management
        for (pos, &char_class) in char_classes.iter().enumerate() {
            self.context_state.curr_class = char_class;

            // Apply pair table rules for break determination
            let break_action = apply_pair_table_rules(pos, &char_classes, &self.context_state);

            if break_action != super::types::BreakClass::Prohibited {
                let priority = calculate_break_priority(char_class, &text_chars[pos]);
                let penalty = calculate_break_penalty(pos, &char_classes);

                opportunities.push(BreakOpportunity {
                    position: pos,
                    break_class: break_action,
                    priority,
                    penalty,
                });
            }

            self.context_state.advance(char_class);
        }

        // Filter and optimize break opportunities
        self.optimize_break_opportunities(opportunities)
    }

    /// Extract characters from shaped glyphs for line break analysis
    fn extract_characters_from_run(&self, run: &ShapedRun) -> Result<Vec<char>, ShapingError> {
        let mut chars = Vec::with_capacity(run.glyphs.len());

        // Reconstruct text from glyph cluster information
        for glyph in &run.glyphs {
            // For shaped text, we need to map back to original characters
            // This is a simplified approach - full implementation would maintain
            // character-to-glyph mapping throughout shaping pipeline
            if let Some(ch) = char::from_u32(glyph.glyph_id as u32) {
                chars.push(ch);
            } else {
                // Default to space for unmappable glyphs
                chars.push(' ');
            }
        }

        Ok(chars)
    }

    /// Optimize break opportunities by removing suboptimal breaks
    fn optimize_break_opportunities(
        &self,
        mut opportunities: Vec<BreakOpportunity>,
    ) -> Result<Vec<BreakOpportunity>, ShapingError> {
        // Sort by position for processing
        opportunities.sort_by_key(|op| op.position);

        // Remove redundant break opportunities
        opportunities.dedup_by_key(|op| op.position);

        // Filter out low-quality breaks near high-quality ones
        let mut filtered = Vec::with_capacity(opportunities.len());
        let mut last_high_quality_pos = None;

        for opp in opportunities {
            let should_keep = match last_high_quality_pos {
                Some(last_pos) if opp.priority >= super::types::BreakPriority::High => {
                    let distance = opp.position.saturating_sub(last_pos);
                    distance >= 5 // Minimum distance between high-quality breaks
                }
                _ => true,
            };

            if should_keep {
                if opp.priority >= super::types::BreakPriority::High {
                    last_high_quality_pos = Some(opp.position);
                }
                filtered.push(opp);
            }
        }

        Ok(filtered)
    }
}
