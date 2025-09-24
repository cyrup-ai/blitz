//! Bidirectional text processing and analysis
//!
//! This module provides fast bidirectional text analysis with optimized
//! direction detection and bidi run extraction.

use unicode_bidi::{BidiClass, BidiInfo, Level};

use super::caching::CacheManager;
use crate::error::ShapingError;
use crate::types::{BidiRun, TextDirection};

/// Bidirectional text processor with performance optimizations
pub struct BidiProcessor;

impl BidiProcessor {
    /// Fast base direction determination with early exit optimization
    pub fn determine_base_direction_fast(text: &str) -> TextDirection {
        let mut first_strong = None;
        let mut char_count = 0;

        // Early exit after checking first 100 characters for performance
        for ch in text.chars().take(100) {
            char_count += 1;
            let bidi_class = CacheManager::get_bidi_class_cached(ch);

            match bidi_class {
                BidiClass::L => {
                    if first_strong.is_none() {
                        first_strong = Some(TextDirection::LeftToRight);
                        // For performance, return immediately on first strong LTR
                        if char_count > 10 {
                            return TextDirection::LeftToRight;
                        }
                    }
                }
                BidiClass::R | BidiClass::AL => {
                    // RTL characters are less common, so always set and continue
                    first_strong = Some(TextDirection::RightToLeft);
                }
                _ => {}
            }
        }

        first_strong.unwrap_or(TextDirection::LeftToRight)
    }

    /// Fast bidirectional processing check with early exit
    pub fn requires_bidi_processing_fast(text: &str) -> bool {
        // Check only first 200 characters for performance
        text.chars().take(200).any(|ch| {
            matches!(
                CacheManager::get_bidi_class_cached(ch),
                BidiClass::R | BidiClass::AL | BidiClass::RLE | BidiClass::RLO
            )
        })
    }

    /// Process bidirectional text with optimized caching
    pub fn process_bidi<'a>(
        text: &'a str,
        base_direction: TextDirection,
    ) -> Result<BidiInfo<'a>, ShapingError> {
        let initial_level = match base_direction {
            TextDirection::RightToLeft => Level::rtl(),
            _ => Level::ltr(),
        };

        // BidiInfo doesn't allocate much, so we don't cache it for performance
        Ok(BidiInfo::new(text, Some(initial_level)))
    }

    /// Extract bidi runs from BidiInfo with zero extra allocation
    #[inline]
    pub fn extract_bidi_runs(
        bidi_info: &BidiInfo,
        para_range: std::ops::Range<usize>,
    ) -> Vec<BidiRun> {
        // Find the paragraph that contains our range using safe error handling
        let paragraph = match bidi_info
            .paragraphs
            .iter()
            .find(|p| p.range.start <= para_range.start && p.range.end >= para_range.end)
        {
            Some(paragraph) => paragraph,
            None => {
                // No paragraph found for range - return empty result instead of panicking
                // This can happen with malformed text or empty input
                return Vec::new();
            }
        };

        // Extract visual runs using correct API - returns (levels, runs) tuple
        let (levels, runs) = bidi_info.visual_runs(paragraph, para_range.clone());

        // Convert to our BidiRun format
        runs.into_iter()
            .map(|range| {
                let level = levels[range.start];
                BidiRun::new(range.start, range.end, level)
            })
            .collect()
    }
}
