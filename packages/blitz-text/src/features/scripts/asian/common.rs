//! Common constants and utilities for Asian script features
//!
//! This module provides shared constants and helper utilities used across
//! all Asian script feature implementations for zero-allocation performance.

/// Standard stylistic sets supported across all Asian scripts
pub const STANDARD_STYLISTIC_SETS: &[u8] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
];

/// Standard stylistic set OpenType features (ss01-ss20)
pub const STANDARD_SS_FEATURES: &[(&str, u32); 20] = &[
    ("ss01", 1),
    ("ss02", 1),
    ("ss03", 1),
    ("ss04", 1),
    ("ss05", 1),
    ("ss06", 1),
    ("ss07", 1),
    ("ss08", 1),
    ("ss09", 1),
    ("ss10", 1),
    ("ss11", 1),
    ("ss12", 1),
    ("ss13", 1),
    ("ss14", 1),
    ("ss15", 1),
    ("ss16", 1),
    ("ss17", 1),
    ("ss18", 1),
    ("ss19", 1),
    ("ss20", 1),
];

/// Base OpenType features common to most Asian scripts
pub const BASE_FEATURES: &[(&str, u32); 4] = &[("kern", 1), ("liga", 1), ("calt", 1), ("locl", 1)];
