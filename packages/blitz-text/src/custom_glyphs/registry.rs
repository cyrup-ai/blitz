//! Custom glyph registry management
//!
//! This module provides lock-free registry operations for custom glyphs
//! using ArcSwap for atomic operations and high-performance lookups.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::Arc;

use ahash::AHashMap;
use arc_swap::ArcSwap;
use glyphon::{CustomGlyph, CustomGlyphId};

use super::types::{CustomGlyphData, CustomGlyphError, GlyphKey, GlyphSystemStats};

thread_local! {
    static GLYPH_BUFFER: RefCell<Vec<CustomGlyph>> = RefCell::new(Vec::with_capacity(256));
    static GLYPHON_GLYPH_BUFFER: RefCell<Vec<glyphon::CustomGlyph>> = RefCell::new(Vec::with_capacity(256));
    static UNICODE_BUFFER: RefCell<Vec<char>> = RefCell::new(Vec::with_capacity(256));
    static ATLAS_COORD_BUFFER: RefCell<Vec<(u32, u32, u32, u32)>> = RefCell::new(Vec::with_capacity(256));
}

/// Lock-free custom glyph registry using ArcSwap for blazing-fast atomic operations
#[derive(Debug)]
pub struct CustomGlyphRegistry {
    glyphs: ArcSwap<AHashMap<GlyphKey, CustomGlyphData>>,
    glyph_id_lookup: ArcSwap<AHashMap<CustomGlyphId, GlyphKey>>,
    next_id: AtomicU16,
    hit_count: AtomicU32,
    miss_count: AtomicU32,
}

impl CustomGlyphRegistry {
    /// Create a new lock-free custom glyph registry
    pub fn new() -> Self {
        Self {
            glyphs: ArcSwap::new(Arc::new(AHashMap::with_capacity(1024))),
            glyph_id_lookup: ArcSwap::new(Arc::new(AHashMap::with_capacity(1024))),
            next_id: AtomicU16::new(1),
            hit_count: AtomicU32::new(0),
            miss_count: AtomicU32::new(0),
        }
    }

    /// Register a new custom glyph with atomic operations
    pub fn register_glyph(
        &self,
        key: GlyphKey,
        glyph_data: CustomGlyphData,
    ) -> Result<CustomGlyphId, CustomGlyphError> {
        let glyph_id = self.next_id.fetch_add(1, Ordering::Relaxed) as CustomGlyphId;

        // Create new glyph with assigned ID
        let mut new_glyph = glyph_data.glyph.clone();
        new_glyph.id = glyph_id;

        let new_glyph_data = CustomGlyphData {
            glyph: new_glyph,
            atlas_coords: glyph_data.atlas_coords,
            metrics: glyph_data.metrics,
            access_count: AtomicU32::new(0),
            last_used_ns: AtomicU32::new(0),
        };

        // Atomic update of both maps
        loop {
            let current_glyphs = self.glyphs.load();
            let current_lookup = self.glyph_id_lookup.load();

            let mut new_glyphs = (**current_glyphs).clone();
            let mut new_lookup = (**current_lookup).clone();

            new_glyphs.insert(key.clone(), new_glyph_data.clone());
            new_lookup.insert(glyph_id, key.clone());

            // Try to swap both atomically
            let glyphs_success = self
                .glyphs
                .compare_and_swap(&current_glyphs, Arc::new(new_glyphs));
            if Arc::ptr_eq(&glyphs_success, &current_glyphs) {
                // First swap succeeded, now try second
                let lookup_success = self
                    .glyph_id_lookup
                    .compare_and_swap(&current_lookup, Arc::new(new_lookup));
                if Arc::ptr_eq(&lookup_success, &current_lookup) {
                    // Both swaps succeeded
                    break;
                }
                // Second swap failed, need to retry both
            }
            // Retry the entire operation
        }

        Ok(glyph_id)
    }

    /// Get glyph data by key with cache hit tracking
    pub fn get_glyph(&self, key: &GlyphKey) -> Option<CustomGlyphData> {
        let glyphs = self.glyphs.load();

        if let Some(glyph_data) = glyphs.get(key) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            glyph_data.record_access();
            Some(glyph_data.clone())
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Get glyph data by ID
    pub fn get_glyph_by_id(&self, id: CustomGlyphId) -> Option<CustomGlyphData> {
        let lookup = self.glyph_id_lookup.load();

        if let Some(key) = lookup.get(&id) {
            self.get_glyph(key)
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Check if glyph exists by key
    pub fn contains_glyph(&self, key: &GlyphKey) -> bool {
        let glyphs = self.glyphs.load();
        glyphs.contains_key(key)
    }

    /// Get all registered glyph keys
    pub fn get_all_keys(&self) -> Vec<GlyphKey> {
        let glyphs = self.glyphs.load();
        glyphs.keys().cloned().collect()
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> GlyphSystemStats {
        let glyphs = self.glyphs.load();
        let hit_count = self.hit_count.load(Ordering::Relaxed);
        let miss_count = self.miss_count.load(Ordering::Relaxed);

        GlyphSystemStats {
            total_glyphs: glyphs.len() as u32,
            cache_hits: hit_count,
            cache_misses: miss_count,
            atlas_utilization: 0.0, // Would be calculated based on atlas usage
        }
    }

    /// Clear all glyphs from registry
    pub fn clear(&self) {
        self.glyphs.store(Arc::new(AHashMap::new()));
        self.glyph_id_lookup.store(Arc::new(AHashMap::new()));
        self.next_id.store(1, Ordering::Relaxed);
        self.hit_count.store(0, Ordering::Relaxed);
        self.miss_count.store(0, Ordering::Relaxed);
    }

    /// Remove glyph by key
    pub fn remove_glyph(&self, key: &GlyphKey) -> Option<CustomGlyphData> {
        loop {
            let current_glyphs = self.glyphs.load();
            let current_lookup = self.glyph_id_lookup.load();

            let glyph_data = current_glyphs.get(key)?;
            let glyph_id = glyph_data.glyph.id;

            let mut new_glyphs = (**current_glyphs).clone();
            let mut new_lookup = (**current_lookup).clone();

            let removed_data = new_glyphs.remove(key);
            new_lookup.remove(&glyph_id);

            // Try to swap both atomically
            let glyphs_success = self
                .glyphs
                .compare_and_swap(&current_glyphs, Arc::new(new_glyphs));
            if Arc::ptr_eq(&glyphs_success, &current_glyphs) {
                let lookup_success = self
                    .glyph_id_lookup
                    .compare_and_swap(&current_lookup, Arc::new(new_lookup));
                if Arc::ptr_eq(&lookup_success, &current_lookup) {
                    return removed_data;
                }
            }
            // Retry if atomic operations failed
        }
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f32 {
        let hits = self.hit_count.load(Ordering::Relaxed);
        let misses = self.miss_count.load(Ordering::Relaxed);
        let total = hits + misses;

        if total > 0 {
            hits as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Get total number of registered glyphs
    pub fn glyph_count(&self) -> usize {
        let glyphs = self.glyphs.load();
        glyphs.len()
    }

    /// Update glyph data atomically
    pub fn update_glyph(
        &self,
        key: &GlyphKey,
        new_data: CustomGlyphData,
    ) -> Result<(), CustomGlyphError> {
        loop {
            let current_glyphs = self.glyphs.load();

            if !current_glyphs.contains_key(key) {
                return Err(CustomGlyphError::GlyphNotFound(key.clone()));
            }

            let mut new_glyphs = (**current_glyphs).clone();
            new_glyphs.insert(key.clone(), new_data.clone());

            let success = self
                .glyphs
                .compare_and_swap(&current_glyphs, Arc::new(new_glyphs));
            if Arc::ptr_eq(&success, &current_glyphs) {
                return Ok(());
            }
            // Retry if atomic operation failed
        }
    }

    /// Batch register multiple glyphs
    pub fn batch_register_glyphs(
        &self,
        glyphs: Vec<(GlyphKey, CustomGlyphData)>,
    ) -> Result<Vec<CustomGlyphId>, CustomGlyphError> {
        let mut glyph_ids = Vec::with_capacity(glyphs.len());

        loop {
            let current_glyphs = self.glyphs.load();
            let current_lookup = self.glyph_id_lookup.load();

            let mut new_glyphs = (**current_glyphs).clone();
            let mut new_lookup = (**current_lookup).clone();
            glyph_ids.clear();

            for (key, mut glyph_data) in glyphs.iter().cloned() {
                let glyph_id = self.next_id.fetch_add(1, Ordering::Relaxed) as CustomGlyphId;
                glyph_data.glyph.id = glyph_id;

                new_glyphs.insert(key.clone(), glyph_data);
                new_lookup.insert(glyph_id, key);
                glyph_ids.push(glyph_id);
            }

            // Try to swap both atomically
            let glyphs_success = self
                .glyphs
                .compare_and_swap(&current_glyphs, Arc::new(new_glyphs));
            if Arc::ptr_eq(&glyphs_success, &current_glyphs) {
                let lookup_success = self
                    .glyph_id_lookup
                    .compare_and_swap(&current_lookup, Arc::new(new_lookup));
                if Arc::ptr_eq(&lookup_success, &current_lookup) {
                    return Ok(glyph_ids);
                }
            }
            // Retry if atomic operations failed
        }
    }

    /// Get glyphs by access pattern (most/least used)
    pub fn get_glyphs_by_usage(
        &self,
        most_used: bool,
        limit: usize,
    ) -> Vec<(GlyphKey, CustomGlyphData)> {
        let glyphs = self.glyphs.load();
        let mut glyph_vec: Vec<_> = glyphs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        glyph_vec.sort_by(|a, b| {
            let a_count = a.1.access_count();
            let b_count = b.1.access_count();

            if most_used {
                b_count.cmp(&a_count)
            } else {
                a_count.cmp(&b_count)
            }
        });

        glyph_vec.into_iter().take(limit).collect()
    }

    /// Cleanup old unused glyphs
    pub fn cleanup_unused_glyphs(&self, max_age_seconds: u32) -> usize {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        let mut removed_count = 0;
        let glyphs = self.glyphs.load();

        let keys_to_remove: Vec<_> = glyphs
            .iter()
            .filter(|(_, data)| {
                let last_used = data.last_used();
                last_used > 0 && (current_time - last_used) > max_age_seconds
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            if self.remove_glyph(&key).is_some() {
                removed_count += 1;
            }
        }

        removed_count
    }

    /// Alias for get_glyph for compatibility
    pub fn get_glyph_data(&self, key: &GlyphKey) -> Option<CustomGlyphData> {
        self.get_glyph(key)
    }

    /// Alias for register_glyph for compatibility
    pub fn store_glyph_data(
        &self,
        key: GlyphKey,
        data: CustomGlyphData,
    ) -> Result<CustomGlyphId, String> {
        self.register_glyph(key, data).map_err(|e| e.to_string())
    }

    /// Get registry size (number of glyphs)
    pub fn size(&self) -> usize {
        let glyphs = self.glyphs.load();
        glyphs.len()
    }
}

impl Default for CustomGlyphRegistry {
    fn default() -> Self {
        Self::new()
    }
}
