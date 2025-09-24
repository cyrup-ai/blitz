//! BiDi processing validation and statistics
//!
//! This module handles validation of BiDi processing results and statistics collection.

use super::super::types::{BidiError, ProcessedBidi};

/// BiDi processing statistics
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl ProcessingStats {
    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f64 / total as f64
        } else {
            0.0
        }
    }
}

/// BiDi processing validation utilities
pub struct BidiValidator;

impl BidiValidator {
    /// Validate BiDi processing result
    pub fn validate_processed_bidi(processed: &ProcessedBidi) -> Result<(), BidiError> {
        // Check that mappings are consistent
        if processed.logical_to_visual.len() != processed.text.chars().count() {
            return Err(BidiError::ProcessingFailed(
                "Logical to visual mapping length mismatch".to_string(),
            ));
        }

        if processed.visual_to_logical.len() != processed.text.chars().count() {
            return Err(BidiError::ProcessingFailed(
                "Visual to logical mapping length mismatch".to_string(),
            ));
        }

        // Check that visual runs cover the entire text
        let mut covered_chars = 0;
        for run in &processed.visual_runs {
            covered_chars += run.end_index - run.start_index;
        }

        if covered_chars != processed.text.len() {
            return Err(BidiError::ProcessingFailed(
                "Visual runs do not cover entire text".to_string(),
            ));
        }

        Ok(())
    }
}
