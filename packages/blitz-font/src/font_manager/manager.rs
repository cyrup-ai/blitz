use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use url::Url;

#[cfg(feature = "web-fonts")]
use crate::WebFontLoader;
use crate::font_manager::discovery_ops::DiscoveryOps;
use crate::font_manager::loading_ops::FontLoadingOps;
use crate::font_manager::registry::RegistryManager;
use crate::{FontError, FontKey, LoadedFont, SystemFont};

#[derive(Debug)]
enum FontLoadRequest {
    SystemFont {
        path: std::path::PathBuf,
        respond_to: tokio::sync::oneshot::Sender<Result<FontKey, FontError>>,
    },
    MemoryFont {
        data: Vec<u8>,
        key: FontKey,
        respond_to: tokio::sync::oneshot::Sender<Result<(), FontError>>,
    },
    WebFont {
        url: Url,
        respond_to: tokio::sync::oneshot::Sender<Result<FontKey, FontError>>,
    },
}

/// Comprehensive lock-free font management system
pub struct FontManager {
    font_system: Arc<Mutex<glyphon::cosmyc_text::FontSystem>>,
    registry_manager: RegistryManager,
    font_count: AtomicUsize,
    system_fonts_loaded: AtomicBool,
    max_cache_size: usize,
    cache_ttl: std::time::Duration,

    #[cfg(feature = "web-fonts")]
    web_font_loader: WebFontLoader,

    // Async communication channels
    font_load_tx: tokio::sync::mpsc::UnboundedSender<FontLoadRequest>,
    font_load_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<FontLoadRequest>>>,
}

impl FontManager {
    /// Create a new FontManager instance with optimal defaults
    pub fn new() -> Result<Self, FontError> {
        Self::with_config(crate::FontManagerBuilder::new())
    }

    /// Create FontManager with custom configuration
    pub fn with_config(config: crate::FontManagerBuilder) -> Result<Self, FontError> {
        let font_system = Arc::new(Mutex::new(glyphon::cosmyc_text::FontSystem::new()));
        let registry_manager = RegistryManager::new();

        #[cfg(feature = "web-fonts")]
        let web_font_loader = WebFontLoader::new()?;

        let (font_load_tx, font_load_rx) = tokio::sync::mpsc::unbounded_channel();

        let manager = Self {
            font_system,
            registry_manager,
            font_count: AtomicUsize::new(0),
            system_fonts_loaded: AtomicBool::new(false),
            max_cache_size: config.max_cache_size,
            cache_ttl: config.cache_ttl,

            #[cfg(feature = "web-fonts")]
            web_font_loader,

            font_load_tx,
            font_load_rx: Arc::new(tokio::sync::Mutex::new(font_load_rx)),
        };

        // Initialize system fonts if requested
        if config.discover_system_fonts {
            DiscoveryOps::initialize_system_fonts(
                &manager.registry_manager,
                &manager.font_system,
                &manager.font_count,
                &manager.system_fonts_loaded,
            )?;
        }

        // Setup fallback chains if requested
        if config.setup_fallbacks {
            DiscoveryOps::setup_default_fallbacks(&manager.registry_manager)?;
        }

        // Add custom fallback chains
        for (family, fallbacks) in config.custom_fallbacks {
            DiscoveryOps::register_fallback_chain(&manager.registry_manager, family, fallbacks)?;
        }

        // Start background font loading task
        manager.start_font_loading_task();

        Ok(manager)
    }

    /// Get the underlying FontSystem for cosmyc-text integration
    pub fn get_font_system(&self) -> Arc<Mutex<glyphon::cosmyc_text::FontSystem>> {
        Arc::clone(&self.font_system)
    }

    /// Get the number of loaded fonts
    pub fn loaded_font_count(&self) -> usize {
        self.font_count.load(Ordering::Acquire)
    }

    /// Start the background font loading task
    fn start_font_loading_task(&self) {
        let rx = Arc::clone(&self.font_load_rx);
        let registry_manager = self.registry_manager.clone();
        let font_system = Arc::clone(&self.font_system);

        tokio::spawn(async move {
            let mut rx = rx.lock().await;
            while let Some(request) = rx.recv().await {
                match request {
                    FontLoadRequest::SystemFont { path, respond_to } => {
                        let result = FontLoadingOps::load_system_font_async(
                            &registry_manager,
                            Arc::clone(&font_system),
                            path,
                        )
                        .await;
                        let _ = respond_to.send(result);
                    }
                    FontLoadRequest::MemoryFont {
                        data,
                        key,
                        respond_to,
                    } => {
                        let result = FontLoadingOps::load_memory_font_async(
                            &registry_manager,
                            Arc::clone(&font_system),
                            data,
                            key,
                        )
                        .await;
                        let _ = respond_to.send(result);
                    }
                    FontLoadRequest::WebFont { url, respond_to } => {
                        let result = FontLoadingOps::load_web_font_async(
                            &registry_manager,
                            Arc::clone(&font_system),
                            url,
                        )
                        .await;
                        let _ = respond_to.send(result);
                    }
                }
            }
        });
    }
}

impl Default for FontManager {
    fn default() -> Self {
        // Provide a fallback implementation that cannot fail
        // Create minimal FontManager without system font discovery to avoid potential failures
        let font_system = Arc::new(Mutex::new(glyphon::cosmyc_text::FontSystem::new()));
        let registry_manager = RegistryManager::new();

        #[cfg(feature = "web-fonts")]
        let web_font_loader = WebFontLoader::minimal();

        let (font_load_tx, font_load_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            font_system,
            registry_manager,
            font_count: AtomicUsize::new(0),
            system_fonts_loaded: AtomicBool::new(false),
            max_cache_size: crate::constants::MAX_FONT_CACHE_SIZE,
            cache_ttl: std::time::Duration::from_secs(crate::constants::DEFAULT_CACHE_TTL_SECONDS),

            #[cfg(feature = "web-fonts")]
            web_font_loader,

            font_load_tx,
            font_load_rx: Arc::new(tokio::sync::Mutex::new(font_load_rx)),
        }
    }
}

impl FontManager {
    /// Find the best matching font for the requested FontKey
    /// Only returns fonts that are registered with FontSystem and usable for text rendering
    pub async fn find_best_font(&self, requested: &FontKey) -> Option<LoadedFont> {
        let registry = self.registry_manager.get_registry();
        let loaded_fonts = &registry.loaded_fonts;

        // First try exact match with FontSystem verification
        if let Some(font) = loaded_fonts.get(requested) {
            if self.verify_font_system_registration(font) {
                font.increment_usage();
                return Some(font.clone());
            }
        }

        // Find best scoring match using the font matching algorithm
        let mut best_match: Option<(&FontKey, &LoadedFont)> = None;
        let mut best_score = u32::MAX;

        for (key, font) in loaded_fonts.iter() {
            // Only consider fonts registered with FontSystem
            if !self.verify_font_system_registration(font) {
                continue;
            }

            // Check family compatibility
            if !crate::utils::are_font_families_compatible(&key.family, &requested.family) {
                continue;
            }

            let score = crate::utils::calculate_font_match_score(key, requested);
            if score < best_score {
                best_score = score;
                best_match = Some((key, font));
            }
        }

        // If no exact family match, try fallback chains
        if best_match.is_none() {
            if let Some(fallback_chain) = registry.fallback_chains.get(&requested.family) {
                for fallback_key in fallback_chain {
                    if let Some(font) = loaded_fonts.get(fallback_key) {
                        if self.verify_font_system_registration(font) {
                            font.increment_usage();
                            return Some(font.clone());
                        }
                    }
                }
            }
        }

        if let Some((_, font)) = best_match {
            font.increment_usage();
            Some(font.clone())
        } else {
            None
        }
    }

    /// Load a web font from URL and return FontKey when ready
    #[cfg(feature = "web-fonts")]
    pub async fn load_web_font(&self, url: Url) -> Result<FontKey, FontError> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let request = FontLoadRequest::WebFont {
            url,
            respond_to: tx,
        };

        self.font_load_tx
            .send(request)
            .map_err(|_| FontError::LoadFailed("Font loading task not running".to_string()))?;

        rx.await
            .map_err(|_| FontError::LoadFailed("Font loading task dropped".to_string()))?
    }

    /// Load font from memory data
    pub async fn load_font_from_data(&self, data: Vec<u8>, key: FontKey) -> Result<(), FontError> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let request = FontLoadRequest::MemoryFont {
            data,
            key,
            respond_to: tx,
        };

        self.font_load_tx
            .send(request)
            .map_err(|_| FontError::LoadFailed("Font loading task not running".to_string()))?;

        rx.await
            .map_err(|_| FontError::LoadFailed("Font loading task dropped".to_string()))?
    }

    /// Get all discovered system fonts that are registered with FontSystem
    /// Only returns fonts that are verified usable for text rendering via cosmyc-text
    pub async fn get_system_fonts(&self) -> Vec<SystemFont> {
        let registry = self.registry_manager.get_registry();
        let mut usable_fonts = Vec::new();

        // Only return system fonts that have corresponding LoadedFont entries registered with FontSystem
        for system_font in registry.system_fonts.iter() {
            let font_key = system_font.to_font_key();
            if let Some(loaded_font) = registry.loaded_fonts.get(&font_key) {
                if self.verify_font_system_registration(loaded_font) {
                    usable_fonts.push(system_font.clone());
                }
            }
        }

        usable_fonts
    }

    /// Get all available font families from fonts registered with FontSystem
    /// Only returns families that are verified usable for text rendering via cosmyc-text
    pub async fn get_font_families(&self) -> Vec<String> {
        let registry = self.registry_manager.get_registry();
        let mut families: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Collect families only from loaded fonts that are registered with FontSystem
        for (key, font) in registry.loaded_fonts.iter() {
            if self.verify_font_system_registration(font) {
                families.insert(key.family.clone());
            }
        }

        let mut family_list: Vec<String> = families.into_iter().collect();
        family_list.sort();
        family_list
    }

    /// Verify that a LoadedFont is properly registered with FontSystem and usable for text rendering
    #[inline]
    fn verify_font_system_registration(&self, font: &LoadedFont) -> bool {
        // Check if font has a valid FontSystem font_id
        if font.font_id.is_none() {
            return false;
        }

        // Additional verification: Check if the font_id confirmed exists in FontSystem database
        // This catches cases where font was registered but later removed from FontSystem
        if let Ok(font_system) = self.font_system.lock() {
            if let Some(font_id) = font.font_id {
                // Verify the font ID exists in FontSystem database
                // We use the font's face_index to validate registration
                font_system.db().face(font_id).is_some()
            } else {
                false
            }
        } else {
            // If we can't acquire the lock, assume font is not available
            false
        }
    }
}

impl Drop for FontManager {
    fn drop(&mut self) {
        // Channel cleanup is automatic
    }
}
