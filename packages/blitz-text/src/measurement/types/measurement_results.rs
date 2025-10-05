//! Measurement result type definitions
//!
//! This module contains types for representing text measurement results,
//! including line measurements and complete text measurements.

use goldylox::traits::{CacheValue, CacheValueMetadata, CompressionHint};
use serde::{Deserialize, Serialize};

use crate::measurement::types::bounds_types::{CharacterPosition, TextBounds};

/// Measurements for a single line of text
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineMeasurement {
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub baseline_offset: f32,
    pub character_positions: Vec<CharacterPosition>,
    pub start_char: usize,
    pub end_char: usize,
    pub glyph_count: usize,
}

impl Default for LineMeasurement {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            ascent: 0.0,
            descent: 0.0,
            line_gap: 0.0,
            baseline_offset: 0.0,
            character_positions: Vec::new(),
            start_char: 0,
            end_char: 0,
            glyph_count: 0,
        }
    }
}

/// Complete text measurement result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TextMeasurement {
    pub content_width: f32,
    pub content_height: f32,
    pub line_height: f32,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub x_height: f32,
    pub cap_height: f32,
    pub advance_width: f32,
    pub bounds: TextBounds,
    pub line_measurements: Vec<LineMeasurement>,
    pub total_character_count: usize,
    pub baseline_offset: f32,
    pub measured_at: u64, // Unix timestamp in milliseconds
}



impl CacheValue for TextMeasurement {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<TextMeasurement>()
            + self.line_measurements.len() * std::mem::size_of::<LineMeasurement>()
            + self
                .line_measurements
                .iter()
                .map(|line| {
                    line.character_positions.len() * std::mem::size_of::<CharacterPosition>()
                })
                .sum::<usize>()
    }

    fn is_expensive(&self) -> bool {
        self.line_measurements.len() > 10 // Large text measurements are expensive
    }

    fn compression_hint(&self) -> CompressionHint {
        CompressionHint::Auto
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

// Metadata type for TextMeasurement
#[derive(Debug, Clone)]
pub struct TextMeasurementMetadata {
    pub access_count: u64,
    pub creation_time: std::time::SystemTime,
    pub last_access: std::time::SystemTime,
}

unsafe impl Send for TextMeasurement {}
unsafe impl Sync for TextMeasurement {}
