//! BiDi text processing module
//!
//! This module provides bidirectional text processing functionality,
//! decomposed into logical separation of concerns.

pub mod analysis;
pub mod core;
pub mod direction;
pub mod validation;

// Re-export main types and functions for backward compatibility
pub use core::BidiProcessor;

pub use analysis::BidiAnalyzer;
pub use direction::DirectionDetector;
// Re-export utility functions as module-level functions
pub use direction::DirectionDetector as DirectionUtils;
pub use validation::BidiValidator as ValidationUtils;
pub use validation::{BidiValidator, ProcessingStats};

// Static utility functions
pub fn has_bidi_content(text: &str) -> bool {
    direction::DirectionDetector::has_bidi_content(text)
}

pub fn get_paragraph_level(
    text: &str,
    base_direction: super::types::Direction,
) -> Result<u8, super::types::BidiError> {
    direction::DirectionDetector::get_paragraph_level(text, base_direction)
}

pub fn split_paragraphs(text: &str) -> Vec<&str> {
    direction::DirectionDetector::split_paragraphs(text)
}

pub fn validate_processed_bidi(
    processed: &super::types::ProcessedBidi,
) -> Result<(), super::types::BidiError> {
    validation::BidiValidator::validate_processed_bidi(processed)
}
