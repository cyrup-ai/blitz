//! Statistics and monitoring functionality
//!
//! This module provides comprehensive statistics gathering and analysis
//! for the text atlas system.

use std::sync::atomic::Ordering;

use super::core::EnhancedTextAtlas;
use super::types::{AtlasStats, MemoryBreakdown};

impl EnhancedTextAtlas {
    /// Get comprehensive atlas statistics
    pub fn get_stats(&self) -> AtlasStats {
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let total_operations = cache_hits + cache_misses;

        AtlasStats {
            cache_hits,
            cache_misses,
            total_operations,
            hit_ratio: if total_operations > 0 {
                cache_hits as f64 / total_operations as f64
            } else {
                0.0
            },
            atlas_growths: self.atlas_growths.load(Ordering::Relaxed),
            trim_operations: self.trim_operations.load(Ordering::Relaxed),
            glyph_allocations: self.glyph_allocations.load(Ordering::Relaxed),
            glyph_deallocations: self.glyph_deallocations.load(Ordering::Relaxed),
            estimated_memory_usage: self.estimated_memory_usage.load(Ordering::Relaxed),
            peak_memory_usage: self.peak_memory_usage.load(Ordering::Relaxed),
            color_atlas_size: self.color_atlas_size.load(Ordering::Relaxed),
            mask_atlas_size: self.mask_atlas_size.load(Ordering::Relaxed),
            stats_duration: self.stats_reset_time.elapsed(),
        }
    }

    /// Get detailed atlas memory breakdown
    pub fn get_memory_breakdown(&self) -> MemoryBreakdown {
        let color_size = self.color_atlas_size.load(Ordering::Relaxed);
        let mask_size = self.mask_atlas_size.load(Ordering::Relaxed);

        // Calculate memory usage (RGBA for color, R for mask)
        let color_memory = (color_size * color_size * 4) as usize; // 4 bytes per pixel
        let mask_memory = (mask_size * mask_size) as usize; // 1 byte per pixel
        let total_atlas_memory = color_memory + mask_memory;

        MemoryBreakdown {
            color_atlas_memory: color_memory,
            mask_atlas_memory: mask_memory,
            total_atlas_memory,
            estimated_overhead: total_atlas_memory / 10, // Estimate 10% overhead
            glyphs_allocated: self.glyph_allocations.load(Ordering::Relaxed),
            avg_memory_per_glyph: if self.glyph_allocations.load(Ordering::Relaxed) > 0 {
                total_atlas_memory as f64 / self.glyph_allocations.load(Ordering::Relaxed) as f64
            } else {
                0.0
            },
        }
    }

    /// Simulate glyph operations for testing/benchmarking
    pub fn simulate_glyph_operations(&self, glyph_count: u32, glyph_size: (u32, u32)) {
        for _ in 0..glyph_count {
            self.track_glyph_allocation(glyphon::ContentType::Color, glyph_size);
        }
        let old_size = 1024 * 1024; // Simulate old atlas size
        let new_size = glyph_size.0 * glyph_size.1 * 4;
        self.track_atlas_growth(glyphon::ContentType::Color, old_size, new_size);
        for _ in 0..glyph_count / 2 {
            self.track_glyph_deallocation(glyphon::ContentType::Color, glyph_size);
        }
    }

    /// Reset all performance statistics
    pub fn reset_stats(&mut self) {
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.atlas_growths.store(0, Ordering::Relaxed);
        self.trim_operations.store(0, Ordering::Relaxed);
        self.glyph_allocations.store(0, Ordering::Relaxed);
        self.glyph_deallocations.store(0, Ordering::Relaxed);

        // Reset peak to current
        let current_memory = self.estimated_memory_usage.load(Ordering::Relaxed);
        self.peak_memory_usage
            .store(current_memory, Ordering::Relaxed);

        self.growth_events.lock().clear();
        self.stats_reset_time = std::time::Instant::now();
    }
}
