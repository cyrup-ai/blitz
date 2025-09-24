//! Thread-local caching for text analysis performance optimization
//!
//! This module provides zero-allocation thread-local caches for script detection,
//! bidirectional class lookup, and analysis result caching.

use unicode_bidi::BidiClass;
use unicode_script::Script;

use crate::types::{ScriptRun, TextAnalysis};

thread_local! {
    pub(super) static SCRIPT_CACHE: std::cell::RefCell<ahash::AHashMap<char, Script>> =
        std::cell::RefCell::new(ahash::AHashMap::new());
}

thread_local! {
    pub(super) static BIDI_CLASS_CACHE: std::cell::RefCell<ahash::AHashMap<char, BidiClass>> =
        std::cell::RefCell::new(ahash::AHashMap::new());
}

thread_local! {
    pub(super) static SCRIPT_RUN_BUFFER: std::cell::RefCell<Vec<ScriptRun>> =
        std::cell::RefCell::new(Vec::with_capacity(16));
}

thread_local! {
    pub(super) static ANALYSIS_CACHE: std::cell::RefCell<ahash::AHashMap<String, TextAnalysis>> =
        std::cell::RefCell::new(ahash::AHashMap::new());
}

/// Cache management utilities
pub struct CacheManager;

impl CacheManager {
    /// Get script for character with zero-allocation thread-local caching
    #[inline]
    pub fn get_script_cached(ch: char) -> Script {
        SCRIPT_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(&script) = cache.get(&ch) {
                script
            } else {
                let script = unicode_script::UnicodeScript::script(&ch);
                // Only cache if under limit to prevent unbounded growth
                if cache.len() < 2000 {
                    cache.insert(ch, script);
                }
                script
            }
        })
    }

    /// Get bidirectional class with zero-allocation thread-local caching
    #[inline]
    pub fn get_bidi_class_cached(ch: char) -> BidiClass {
        BIDI_CLASS_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(&bidi_class) = cache.get(&ch) {
                bidi_class
            } else {
                let bidi_class = unicode_bidi::bidi_class(ch);
                // Only cache if under limit
                if cache.len() < 2000 {
                    cache.insert(ch, bidi_class);
                }
                bidi_class
            }
        })
    }

    /// Get cached analysis result (zero allocation for hits)
    pub fn get_cached_analysis(text: &str) -> Option<TextAnalysis> {
        ANALYSIS_CACHE.with(|cache| cache.borrow().get(text).cloned())
    }

    /// Cache analysis result with size limit
    pub fn cache_analysis(text: String, analysis: TextAnalysis, max_entries: usize) {
        ANALYSIS_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();

            // Prevent unbounded cache growth
            if cache.len() >= max_entries {
                // Simple eviction: clear 25% of entries
                let keys_to_remove: Vec<_> = cache.keys().take(max_entries / 4).cloned().collect();

                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }

            cache.insert(text, analysis);
        });
    }

    /// Clear all thread-local caches
    pub fn clear_all_caches() {
        SCRIPT_CACHE.with(|cache| cache.borrow_mut().clear());
        BIDI_CLASS_CACHE.with(|cache| cache.borrow_mut().clear());
        SCRIPT_RUN_BUFFER.with(|buffer| buffer.borrow_mut().clear());
        ANALYSIS_CACHE.with(|cache| cache.borrow_mut().clear());
    }

    /// Get cache statistics for monitoring (zero allocation)
    pub fn cache_stats() -> (usize, usize, usize, usize) {
        let script_cache_size = SCRIPT_CACHE.with(|cache| cache.borrow().len());
        let bidi_cache_size = BIDI_CLASS_CACHE.with(|cache| cache.borrow().len());
        let analysis_cache_size = ANALYSIS_CACHE.with(|cache| cache.borrow().len());
        let buffer_capacity = SCRIPT_RUN_BUFFER.with(|buffer| buffer.borrow().capacity());

        (
            script_cache_size,
            bidi_cache_size,
            analysis_cache_size,
            buffer_capacity,
        )
    }

    /// Use the script run buffer for zero-allocation script detection
    pub fn with_script_run_buffer<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Vec<ScriptRun>) -> R,
    {
        SCRIPT_RUN_BUFFER.with(|buffer| {
            let mut buffer = buffer.borrow_mut();
            buffer.clear(); // Reuse existing allocation
            f(&mut buffer)
        })
    }
}
