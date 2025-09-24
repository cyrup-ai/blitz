//! Custom glyph types and data structures
//!
//! This module contains all the core types, structs, and enums
//! used throughout the custom glyph system.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::AtomicU32;

pub use cosmyc_text::Buffer;
pub use glyphon::CustomGlyph;
pub use glyphon::CustomGlyphId;
use thiserror::Error;

/// Key for identifying unique glyphs in the registry
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GlyphKey {
    pub unicode_codepoint: u32,
    pub size_class: u8,
    pub style_flags: u8,
    pub color_key: u32,
}

/// Complete glyph data stored in registry
#[derive(Debug)]
pub struct CustomGlyphData {
    pub glyph: CustomGlyph,
    pub atlas_coords: AtlasCoords,
    pub metrics: GlyphMetrics,
    pub access_count: AtomicU32,
    pub last_used_ns: AtomicU32,
}

// Manual Clone implementation to handle AtomicU32
impl Clone for CustomGlyphData {
    fn clone(&self) -> Self {
        Self {
            glyph: self.glyph.clone(),
            atlas_coords: self.atlas_coords,
            metrics: self.metrics,
            access_count: AtomicU32::new(
                self.access_count.load(std::sync::atomic::Ordering::Relaxed),
            ),
            last_used_ns: AtomicU32::new(
                self.last_used_ns.load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

/// GPU texture atlas coordinates
#[derive(Debug, Clone, Copy)]
pub struct AtlasCoords {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Glyph rendering metrics
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub advance_x: f32,
    pub advance_y: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub width: f32,
    pub height: f32,
}

/// Configuration for the custom glyph system
#[derive(Debug, Clone)]
pub struct GlyphSystemConfig {
    pub atlas_size: u32,
    pub max_glyphs: u32,
    pub enable_cache: bool,
    pub cache_ttl_seconds: u32,
}

impl Default for GlyphSystemConfig {
    fn default() -> Self {
        Self {
            atlas_size: 1024,
            max_glyphs: 4096,
            enable_cache: true,
            cache_ttl_seconds: 300, // 5 minutes
        }
    }
}

/// System statistics for monitoring performance
#[derive(Debug)]
pub struct GlyphSystemStats {
    pub total_glyphs: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub atlas_utilization: f32,
}

/// Comprehensive error handling for custom glyph operations
#[derive(Debug, Error)]
pub enum CustomGlyphError {
    #[error("Glyph registry is unavailable")]
    RegistryUnavailable,

    #[error("Failed to register glyph: {0}")]
    RegistrationFailed(String),

    #[error("Glyph not found: {0:?}")]
    GlyphNotFound(GlyphKey),

    #[error("Atlas is full, cannot add more glyphs")]
    AtlasFull,

    #[error("Invalid glyph data: {0}")]
    InvalidGlyphData(String),

    #[error("Invalid range for glyph data: {0}")]
    InvalidRangeGlyphData(String),

    #[error("GPU cache error: {0}")]
    GpuCacheError(String),

    #[error("Atlas decode error: {0}")]
    AtlasDecodeError(String),

    #[error("Coordinate calculation failed for codepoint: {0:X}")]
    CoordinateCalculationFailed(u32),

    #[error("Unsupported glyph format: {0}")]
    UnsupportedFormat(String),

    #[error("Memory allocation failed")]
    MemoryAllocationFailed,

    #[error("Invalid text range")]
    InvalidRange,
}

/// Atlas metadata for coordinate mapping
#[derive(Debug, Clone, Copy)]
pub struct AtlasMetadata {
    /// Total atlas width in pixels
    pub width: u32,
    /// Total atlas height in pixels
    pub height: u32,
    /// Individual glyph width in pixels
    pub glyph_width: u32,
    /// Individual glyph height in pixels
    pub glyph_height: u32,
    /// Number of glyphs per row
    pub glyphs_per_row: u32,
    /// Number of rows in atlas
    pub rows: u32,
}

impl Default for AtlasMetadata {
    #[inline]
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            glyph_width: 32,
            glyph_height: 32,
            glyphs_per_row: 16,
            rows: 16,
        }
    }
}

impl AtlasMetadata {
    /// Create new atlas metadata with specified dimensions
    pub fn new(width: u32, height: u32, glyph_width: u32, glyph_height: u32) -> Self {
        let glyphs_per_row = width / glyph_width;
        let rows = height / glyph_height;

        Self {
            width,
            height,
            glyph_width,
            glyph_height,
            glyphs_per_row,
            rows,
        }
    }

    /// Calculate total number of glyphs that can fit in atlas
    pub fn total_glyphs(&self) -> u32 {
        self.glyphs_per_row * self.rows
    }

    /// Calculate atlas coordinates from glyph index
    pub fn coords_from_index(&self, index: u32) -> Option<AtlasCoords> {
        if index >= self.total_glyphs() {
            return None;
        }

        let row = index / self.glyphs_per_row;
        let col = index % self.glyphs_per_row;

        Some(AtlasCoords {
            x: (col * self.glyph_width) as u16,
            y: (row * self.glyph_height) as u16,
            width: self.glyph_width as u16,
            height: self.glyph_height as u16,
        })
    }

    /// Calculate glyph index from atlas coordinates
    pub fn index_from_coords(&self, coords: &AtlasCoords) -> Option<u32> {
        let col = coords.x as u32 / self.glyph_width;
        let row = coords.y as u32 / self.glyph_height;

        if col >= self.glyphs_per_row || row >= self.rows {
            return None;
        }

        Some(row * self.glyphs_per_row + col)
    }
}

impl GlyphKey {
    /// Create new glyph key from components
    pub fn new(unicode_codepoint: u32, size_class: u8, style_flags: u8, color_key: u32) -> Self {
        Self {
            unicode_codepoint,
            size_class,
            style_flags,
            color_key,
        }
    }

    /// Create glyph key from custom glyph
    pub fn from_custom_glyph(glyph: &CustomGlyph, color_key: u32) -> Self {
        Self::new(
            glyph.id as u32,
            0, // Default size class
            0, // Default style flags
            color_key,
        )
    }

    /// Get hash value for this key
    pub fn hash_value(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl CustomGlyphData {
    /// Create new glyph data
    pub fn new(glyph: CustomGlyph, atlas_coords: AtlasCoords, metrics: GlyphMetrics) -> Self {
        Self {
            glyph,
            atlas_coords,
            metrics,
            access_count: AtomicU32::new(0),
            last_used_ns: AtomicU32::new(0),
        }
    }

    /// Record access to this glyph
    pub fn record_access(&self) {
        self.access_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        self.last_used_ns
            .store(now, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get access count
    pub fn access_count(&self) -> u32 {
        self.access_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get last used timestamp
    pub fn last_used(&self) -> u32 {
        self.last_used_ns.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl GlyphMetrics {
    /// Create new glyph metrics
    pub fn new(
        advance_x: f32,
        advance_y: f32,
        bearing_x: f32,
        bearing_y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            advance_x,
            advance_y,
            bearing_x,
            bearing_y,
            width,
            height,
        }
    }

    /// Create default metrics for a given size
    pub fn default_for_size(size: f32) -> Self {
        Self {
            advance_x: size,
            advance_y: 0.0,
            bearing_x: 0.0,
            bearing_y: size * 0.8,
            width: size,
            height: size,
        }
    }
}

impl GlyphSystemStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self {
            total_glyphs: 0,
            cache_hits: 0,
            cache_misses: 0,
            atlas_utilization: 0.0,
        }
    }

    /// Calculate cache hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Update atlas utilization
    pub fn update_atlas_utilization(&mut self, used_glyphs: u32, max_glyphs: u32) {
        if max_glyphs > 0 {
            self.atlas_utilization = used_glyphs as f32 / max_glyphs as f32;
        }
    }
}
