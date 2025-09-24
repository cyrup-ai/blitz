//! Memory management and tracking
//!
//! This module handles memory tracking, glyph allocation/deallocation,
//! and atlas growth monitoring.

use std::sync::atomic::Ordering;
use std::time::Instant;

use glyphon::ContentType;

use super::core::EnhancedTextAtlas;
use super::types::AtlasGrowthEvent;

impl EnhancedTextAtlas {
    /// Track glyph allocation (called internally when new glyphs are added)
    pub(crate) fn track_glyph_allocation(&self, content_type: ContentType, size: (u32, u32)) {
        self.glyph_allocations.fetch_add(1, Ordering::Relaxed);

        // Estimate memory usage
        let bytes_per_pixel = match content_type {
            ContentType::Color => 4,
            ContentType::Mask => 1,
        };
        let glyph_memory = (size.0 * size.1 * bytes_per_pixel) as usize;

        let new_memory = self
            .estimated_memory_usage
            .fetch_add(glyph_memory, Ordering::Relaxed)
            + glyph_memory;

        // Update peak memory usage
        let mut peak = self.peak_memory_usage.load(Ordering::Relaxed);
        while peak < new_memory {
            match self.peak_memory_usage.compare_exchange_weak(
                peak,
                new_memory,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(current) => peak = current,
            }
        }
    }

    /// Track glyph deallocation (called internally when glyphs are removed)
    pub(crate) fn track_glyph_deallocation(&self, content_type: ContentType, size: (u32, u32)) {
        self.glyph_deallocations.fetch_add(1, Ordering::Relaxed);

        // Estimate memory freed
        let bytes_per_pixel = match content_type {
            ContentType::Color => 4,
            ContentType::Mask => 1,
        };
        let glyph_memory = (size.0 * size.1 * bytes_per_pixel) as usize;

        self.estimated_memory_usage.fetch_sub(
            glyph_memory.min(self.estimated_memory_usage.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
    }

    /// Track atlas growth event
    pub(crate) fn track_atlas_growth(
        &self,
        content_type: ContentType,
        old_size: u32,
        new_size: u32,
    ) {
        self.atlas_growths.fetch_add(1, Ordering::Relaxed);

        // Update atlas size tracking
        match content_type {
            ContentType::Color => {
                self.color_atlas_size.store(new_size, Ordering::Relaxed);
            }
            ContentType::Mask => {
                self.mask_atlas_size.store(new_size, Ordering::Relaxed);
            }
        }

        // Record growth event
        let event = AtlasGrowthEvent {
            timestamp: Instant::now(),
            content_type,
            old_size,
            new_size,
            growth_factor: new_size as f64 / old_size as f64,
        };

        self.growth_events.lock().push(event);

        // Keep only recent growth events (last 100)
        let mut events = self.growth_events.lock();
        if events.len() > 100 {
            let excess = events.len() - 100;
            events.drain(0..excess);
        }
    }

    /// Get atlas growth history
    pub fn get_growth_history(&self) -> Vec<AtlasGrowthEvent> {
        self.growth_events.lock().clone()
    }
}
