use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicPtr, Ordering},
};

use url::Url;

use crate::{FontError, FontKey, LoadedFont, SystemFont, WebFontEntry};

/// Lock-free font registry using immutable data structures
#[derive(Clone)]
pub struct FontRegistry {
    pub loaded_fonts: Arc<HashMap<FontKey, LoadedFont>>,
    pub system_fonts: Arc<Vec<SystemFont>>,
    pub fallback_chains: Arc<HashMap<String, Vec<FontKey>>>,
    pub web_font_cache: Arc<HashMap<Url, WebFontEntry>>,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self {
            loaded_fonts: Arc::new(HashMap::new()),
            system_fonts: Arc::new(Vec::new()),
            fallback_chains: Arc::new(HashMap::new()),
            web_font_cache: Arc::new(HashMap::new()),
        }
    }

    #[inline]
    pub fn with_loaded_fonts(self, fonts: HashMap<FontKey, LoadedFont>) -> Self {
        Self {
            loaded_fonts: Arc::new(fonts),
            ..self
        }
    }

    #[inline]
    pub fn with_system_fonts(self, fonts: Vec<SystemFont>) -> Self {
        Self {
            system_fonts: Arc::new(fonts),
            ..self
        }
    }

    #[inline]
    pub fn with_fallback_chains(self, chains: HashMap<String, Vec<FontKey>>) -> Self {
        Self {
            fallback_chains: Arc::new(chains),
            ..self
        }
    }

    #[inline]
    pub fn with_web_font_cache(self, cache: HashMap<Url, WebFontEntry>) -> Self {
        Self {
            web_font_cache: Arc::new(cache),
            ..self
        }
    }
}

/// Atomic registry operations for lock-free updates
#[derive(Clone)]
pub struct RegistryManager {
    registry: Arc<AtomicPtr<FontRegistry>>,
}

impl RegistryManager {
    pub fn new() -> Self {
        let initial_registry = Box::into_raw(Box::new(FontRegistry::new()));
        Self {
            registry: Arc::new(AtomicPtr::new(initial_registry)),
        }
    }

    /// Get current registry snapshot (atomic load)
    pub fn get_registry(&self) -> FontRegistry {
        let ptr = self.registry.load(Ordering::Acquire);
        unsafe { (*ptr).clone() }
    }

    /// Atomic registry update using compare-and-swap
    pub fn update_registry<F>(&self, update_fn: F) -> Result<(), FontError>
    where
        F: Fn(FontRegistry) -> FontRegistry,
    {
        loop {
            let current_registry = self.get_registry();
            let new_registry = update_fn(current_registry);
            let new_ptr = Box::into_raw(Box::new(new_registry));

            match self.registry.compare_exchange(
                self.registry.load(Ordering::Acquire),
                new_ptr,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(old_ptr) => {
                    // Success - cleanup old registry
                    unsafe {
                        let _ = Box::from_raw(old_ptr);
                    }
                    return Ok(());
                }
                Err(_) => {
                    // CAS failed - cleanup new registry and retry
                    unsafe {
                        let _ = Box::from_raw(new_ptr);
                    }
                }
            }
        }
    }

    /// Add or update a loaded font
    pub fn add_loaded_font(&self, key: FontKey, font: LoadedFont) -> Result<(), FontError> {
        self.update_registry(|registry| {
            let mut fonts = (*registry.loaded_fonts).clone();
            fonts.insert(key.clone(), font.clone());
            registry.with_loaded_fonts(fonts)
        })
    }

    /// Remove a loaded font
    pub fn remove_loaded_font(&self, key: &FontKey) -> Result<(), FontError> {
        self.update_registry(|registry| {
            let mut fonts = (*registry.loaded_fonts).clone();
            fonts.remove(key);
            registry.with_loaded_fonts(fonts)
        })
    }

    /// Update system fonts
    pub fn update_system_fonts(&self, fonts: Vec<SystemFont>) -> Result<(), FontError> {
        self.update_registry(|registry| registry.with_system_fonts(fonts.clone()))
    }

    /// Update fallback chains
    pub fn update_fallback_chains(
        &self,
        chains: HashMap<String, Vec<FontKey>>,
    ) -> Result<(), FontError> {
        self.update_registry(|registry| registry.with_fallback_chains(chains.clone()))
    }

    /// Update web font cache
    pub fn update_web_font_cache(
        &self,
        cache: HashMap<Url, WebFontEntry>,
    ) -> Result<(), FontError> {
        self.update_registry(|registry| registry.with_web_font_cache(cache.clone()))
    }
}

impl Drop for RegistryManager {
    fn drop(&mut self) {
        let ptr = self.registry.load(Ordering::Acquire);
        if !ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
    }
}
