//! Text atlas type definitions
//!
//! This module contains all the type definitions used by the enhanced text atlas system.

use std::time::Instant;

use glyphon::ContentType;

/// Atlas performance statistics
#[derive(Debug, Clone, Copy)]
pub struct AtlasStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_operations: u64,
    pub hit_ratio: f64,
    pub atlas_growths: u32,
    pub trim_operations: u64,
    pub glyph_allocations: u64,
    pub glyph_deallocations: u64,
    pub estimated_memory_usage: usize,
    pub peak_memory_usage: usize,
    pub color_atlas_size: u32,
    pub mask_atlas_size: u32,
    pub stats_duration: std::time::Duration,
}

impl AtlasStats {
    /// Get cache efficiency score (0.0 to 1.0)
    pub fn cache_efficiency(&self) -> f64 {
        self.hit_ratio
    }

    /// Get memory efficiency (current/peak ratio)
    pub fn memory_efficiency(&self) -> f64 {
        if self.peak_memory_usage > 0 {
            self.estimated_memory_usage as f64 / self.peak_memory_usage as f64
        } else {
            1.0
        }
    }

    /// Get growth frequency (growths per hour)
    pub fn growth_frequency(&self) -> f64 {
        let hours = self.stats_duration.as_secs_f64() / 3600.0;
        if hours > 0.0 {
            self.atlas_growths as f64 / hours
        } else {
            0.0
        }
    }

    /// Get glyph turnover rate (deallocations/allocations)
    pub fn glyph_turnover_rate(&self) -> f64 {
        if self.glyph_allocations > 0 {
            self.glyph_deallocations as f64 / self.glyph_allocations as f64
        } else {
            0.0
        }
    }
}

/// Memory usage breakdown
#[derive(Debug, Clone, Copy)]
pub struct MemoryBreakdown {
    pub color_atlas_memory: usize,
    pub mask_atlas_memory: usize,
    pub total_atlas_memory: usize,
    pub estimated_overhead: usize,
    pub glyphs_allocated: u64,
    pub avg_memory_per_glyph: f64,
}

/// Atlas growth prediction
#[derive(Debug, Clone, Copy)]
pub struct GrowthPrediction {
    pub growth_needed: bool,
    pub estimated_additional_memory: usize,
    pub predicted_total_memory: usize,
    pub current_capacity: usize,
    pub confidence: f64, // 0.0 to 1.0
}

/// Atlas growth event tracking
#[derive(Debug, Clone, Copy)]
pub struct AtlasGrowthEvent {
    pub timestamp: Instant,
    pub content_type: ContentType,
    pub old_size: u32,
    pub new_size: u32,
    pub growth_factor: f64,
}

/// Atlas trim event tracking
#[derive(Debug, Clone, Copy)]
pub struct TrimEvent {
    pub timestamp: Instant,
    pub memory_saved: usize,
    pub duration: std::time::Duration,
}

/// Atlas optimization result
#[derive(Debug, Clone, Copy)]
pub struct OptimizationResult {
    pub memory_saved: usize,
    pub optimization_time: std::time::Duration,
    pub glyphs_removed: u32,
    pub fragmentation_reduced: f64, // 0.0 to 1.0
}
