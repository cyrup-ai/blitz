//! Cache key type definitions
//!
//! This module contains cache key types for efficient lookup and storage
//! of measurement results, font metrics, and baseline calculations.

use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use cosmyc_text::fontdb;
use serde::{Deserialize, Serialize};

use crate::measurement::types::baseline_types::CSSBaseline;

/// Cache key for text measurements with perfect hash distribution
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct MeasurementCacheKey {
    pub text_hash: u64,
    pub font_size_bits: u32,
    pub max_width_bits: u32,
    pub font_family_hash: u64,
    pub baseline: CSSBaseline,
}

impl MeasurementCacheKey {
    pub fn new(
        text: &str,
        font_size: f32,
        max_width: Option<f32>,
        font_family: &str,
        baseline: CSSBaseline,
    ) -> Self {
        let mut text_hasher = DefaultHasher::new();
        text.hash(&mut text_hasher);
        let text_hash = text_hasher.finish();

        let mut family_hasher = DefaultHasher::new();
        font_family.hash(&mut family_hasher);
        let font_family_hash = family_hasher.finish();

        Self {
            text_hash,
            font_size_bits: font_size.to_bits(),
            max_width_bits: max_width.unwrap_or(f32::INFINITY).to_bits(),
            font_family_hash,
            baseline,
        }
    }
}

impl Default for MeasurementCacheKey {
    fn default() -> Self {
        Self {
            text_hash: 0,
            font_size_bits: 0,
            max_width_bits: f32::INFINITY.to_bits(),
            font_family_hash: 0,
            baseline: CSSBaseline::Alphabetic,
        }
    }
}

// MeasurementCacheKey no longer implements CacheKey - goldylox uses String keys directly

// MeasurementCacheKey is no longer needed - goldylox uses String keys directly

unsafe impl Send for MeasurementCacheKey {}
unsafe impl Sync for MeasurementCacheKey {}

/// Font metrics cache key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontMetricsCacheKey {
    pub font_id: fontdb::ID,
    pub font_size_bits: u32,
}

impl FontMetricsCacheKey {
    pub fn new(font_id: fontdb::ID, font_size: f32) -> Self {
        Self {
            font_id,
            font_size_bits: font_size.to_bits(),
        }
    }
}

/// Baseline cache key  
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BaselineCacheKey {
    pub font_id: fontdb::ID,
    pub font_size_bits: u32,
    pub baseline: CSSBaseline,
}

impl BaselineCacheKey {
    pub fn new(font_id: fontdb::ID, font_size: f32, baseline: CSSBaseline) -> Self {
        Self {
            font_id,
            font_size_bits: font_size.to_bits(),
            baseline,
        }
    }
}
