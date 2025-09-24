//! BiDi direction detection and level conversion utilities
//!
//! This module handles text direction detection and BiDi level conversions.

use unicode_bidi::Level;

use super::super::types::{BidiError, BidiRenderOptions, Direction};

/// Direction detection and level conversion utilities
pub struct DirectionDetector {
    default_direction: Direction,
}

impl DirectionDetector {
    /// Create new direction detector
    pub fn new(default_direction: Direction) -> Self {
        Self { default_direction }
    }

    /// Determine base text direction from text and options
    pub fn determine_base_direction(
        &self,
        text: &str,
        options: &BidiRenderOptions,
    ) -> Result<Direction, BidiError> {
        match options.base_direction {
            Direction::Auto => Ok(self.detect_paragraph_direction(text)),
            direction => Ok(direction),
        }
    }

    /// Detect paragraph direction from first strong directional character
    pub fn detect_paragraph_direction(&self, text: &str) -> Direction {
        for ch in text.chars() {
            let bidi_class = unicode_bidi::bidi_class(ch);
            match bidi_class {
                unicode_bidi::BidiClass::L => return Direction::LeftToRight,
                unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL => {
                    return Direction::RightToLeft
                }
                _ => continue,
            }
        }
        self.default_direction
    }

    /// Convert Direction to BiDi Level
    pub fn direction_to_level(&self, direction: Direction) -> Result<Level, BidiError> {
        match direction {
            Direction::LeftToRight => Ok(Level::ltr()),
            Direction::RightToLeft => Ok(Level::rtl()),
            Direction::Auto => Ok(Level::ltr()), // Default fallback
        }
    }

    /// Check if text contains bidirectional content
    pub fn has_bidi_content(text: &str) -> bool {
        for ch in text.chars() {
            let bidi_class = unicode_bidi::bidi_class(ch);
            match bidi_class {
                unicode_bidi::BidiClass::R
                | unicode_bidi::BidiClass::AL
                | unicode_bidi::BidiClass::RLE
                | unicode_bidi::BidiClass::RLO
                | unicode_bidi::BidiClass::RLI => return true,
                _ => continue,
            }
        }
        false
    }

    /// Get paragraph embedding level for text
    pub fn get_paragraph_level(text: &str, base_direction: Direction) -> Result<u8, BidiError> {
        let base_level = match base_direction {
            Direction::LeftToRight => Level::ltr(),
            Direction::RightToLeft => Level::rtl(),
            Direction::Auto => {
                // Auto-detect from first strong character
                for ch in text.chars() {
                    let bidi_class = unicode_bidi::bidi_class(ch);
                    match bidi_class {
                        unicode_bidi::BidiClass::L => return Ok(Level::ltr().number()),
                        unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL => {
                            return Ok(Level::rtl().number())
                        }
                        _ => continue,
                    }
                }
                Level::ltr() // Default to LTR
            }
        };

        Ok(base_level.number())
    }

    /// Split text into paragraphs for BiDi processing
    pub fn split_paragraphs(text: &str) -> Vec<&str> {
        text.split('\n').collect()
    }
}
