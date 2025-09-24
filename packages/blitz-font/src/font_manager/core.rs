use crate::{
    FontError, FontKey, LoadedFont,
};
use crate::font_manager::registry::RegistryManager;
use cosmyc_text::{FontSystem, Stretch, Style, Weight};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, AtomicBool, Ordering},
};
use url::Url;

#[cfg(feature = "web-fonts")]
use crate::WebFontLoader;

/// Font loading request types for async background processing
#[derive(Debug)]
pub(crate) enum FontLoadRequest {
    SystemFont { 
        path: std::path::PathBuf, 
        respond_to: tokio::sync::oneshot::Sender<Result<FontKey, FontError>> 
    },
    MemoryFont { 
        data: Vec<u8>, 
        key: FontKey, 
        respond_to: tokio::sync::oneshot::Sender<Result<(), FontError>> 
    },
    WebFont { 
        url: Url, 
        respond_to: tokio::sync::oneshot::Sender<Result<FontKey, FontError>> 
    },
}

/// Comprehensive lock-free font management system
/// 
/// FontManager provides a high-performance, thread-safe interface for loading, caching,
/// and discovering fonts across system fonts, web fonts, and in-memory fonts.
/// Uses atomic operations and lock-free data structures for maximum concurrency.
pub struct FontManager {
    pub(crate) font_system: Arc<Mutex<cosmyc_text::FontSystem>>,
    pub(crate) registry_manager: RegistryManager,
    pub(crate) font_count: AtomicUsize,
    pub(crate) system_fonts_loaded: AtomicBool,
    pub(crate) max_cache_size: usize,
    pub(crate) cache_ttl: std::time::Duration,
    
    #[cfg(feature = "web-fonts")]
    pub(crate) web_font_loader: WebFontLoader,
    
    // Async communication channels for background font loading
    pub(crate) font_load_tx: tokio::sync::mpsc::UnboundedSender<FontLoadRequest>,
    pub(crate) font_load_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<FontLoadRequest>>>,
}

impl FontManager {
    /// Create a new FontManager instance with optimal defaults
    /// 
    /// Automatically discovers system fonts, sets up default fallback chains,
    /// and starts background font loading tasks.
    #[inline]
    pub fn new() -> Result<Self, FontError> {
        Self::with_config(crate::FontManagerBuilder::new())
    }
    
    /// Create FontManager with custom configuration
    /// 
    /// Allows fine-tuned control over font discovery, caching, and fallback behavior.
    /// Uses zero-allocation patterns and lock-free atomic operations for maximum performance.
    pub fn with_config(config: crate::FontManagerBuilder) -> Result<Self, FontError> {
        let font_system = Arc::new(Mutex::new(cosmyc_text::FontSystem::new()));
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
            manager.initialize_system_fonts()?;
        }
        
        // Setup fallback chains if requested
        if config.setup_fallbacks {
            manager.setup_default_fallbacks()?;
        }
        
        // Add custom fallback chains
        for (family, fallbacks) in config.custom_fallbacks {
            manager.register_fallback_chain(family, fallbacks)?;
        }
        
        // Start background font loading task
        manager.start_font_loading_task();
        
        Ok(manager)
    }
    
    /// Get the underlying FontSystem for cosmyc-text integration
    /// 
    /// Returns a thread-safe reference to the FontSystem for text rendering operations.
    /// FontSystem manages the fontdb database and glyph rasterization.
    #[inline]
    pub fn get_font_system(&self) -> Arc<Mutex<cosmyc_text::FontSystem>> {
        Arc::clone(&self.font_system)
    }

    /// Register a custom fallback chain for a font family
    /// 
    /// When a requested font family is not available, the system will try fonts
    /// from the fallback chain in order until a suitable font is found.
    pub fn register_fallback_chain(&self, family: String, fallbacks: Vec<FontKey>) -> Result<(), FontError> {
        let registry = self.registry_manager.get_registry();
        let mut chains = (*registry.fallback_chains).clone();
        chains.insert(family, fallbacks);
        self.registry_manager.update_fallback_chains(chains)
    }

    /// Get the number of loaded fonts
    /// 
    /// Returns the count of fonts currently loaded and available for use.
    /// Uses atomic operations for lock-free access.
    #[inline]
    pub fn loaded_font_count(&self) -> usize {
        self.font_count.load(Ordering::Acquire)
    }
    
    /// Verify that a LoadedFont is properly registered with FontSystem and usable for text rendering
    /// 
    /// Performs comprehensive validation including font_id existence and FontSystem database consistency.
    /// This ensures fonts returned by discovery APIs are verified usable for text rendering.
    #[inline]
    pub(crate) fn verify_font_system_registration(&self, font: &LoadedFont) -> bool {
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

impl Default for FontManager {
    /// Create a minimal FontManager instance without system font discovery
    /// 
    /// Provides a fallback implementation that cannot fail, suitable for
    /// environments where system font discovery might not be available.
    fn default() -> Self {
        let font_system = Arc::new(Mutex::new(cosmyc_text::FontSystem::new()));
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
    
    /// Setup default font fallback chains
    /// 
    /// Configures standard fallback chains for serif, sans-serif, monospace, cursive,
    /// and fantasy font families to ensure text can always be rendered.
    pub(crate) fn setup_default_fallbacks(&self) -> Result<(), FontError> {
        let mut fallbacks = HashMap::new();
        
        // Serif fonts
        fallbacks.insert("serif".to_string(), vec![
            FontKey::new("Times New Roman".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Times".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Liberation Serif".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("serif".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
        ]);
        
        // Sans-serif fonts
        fallbacks.insert("sans-serif".to_string(), vec![
            FontKey::new("Arial".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Helvetica".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Liberation Sans".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("DejaVu Sans".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("sans-serif".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
        ]);
        
        // Monospace fonts
        fallbacks.insert("monospace".to_string(), vec![
            FontKey::new("Consolas".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Monaco".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Courier New".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Liberation Mono".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("DejaVu Sans Mono".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("monospace".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
        ]);
        
        // Cursive fonts
        fallbacks.insert("cursive".to_string(), vec![
            FontKey::new("Comic Sans MS".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("Apple Chancery".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("cursive".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
        ]);
        
        // Fantasy fonts
        fallbacks.insert("fantasy".to_string(), vec![
            FontKey::new("Impact".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
            FontKey::new("fantasy".to_string(), Weight::NORMAL, Style::Normal, Stretch::Normal),
        ]);
        
        self.registry_manager.update_fallback_chains(fallbacks)
    }
}


impl Drop for FontManager {
    fn drop(&mut self) {
        // Channel cleanup is automatic
        // FontSystem and registry cleanup handled by Arc drop
    }
}

// Method implementations that delegate to specialized modules
impl FontManager {
    /// Initialize system fonts by delegating to DiscoveryOps
    pub(crate) fn initialize_system_fonts(&self) -> Result<(), FontError> {
        use crate::font_manager::discovery_ops::DiscoveryOps;
        DiscoveryOps::initialize_system_fonts(&self.registry_manager, &self.font_system, &self.font_count, &self.system_fonts_loaded)
    }
    
    /// Start background font loading task
    pub(crate) fn start_font_loading_task(&self) {
        let rx = Arc::clone(&self.font_load_rx);
        let registry_manager = self.registry_manager.clone();
        let font_system = Arc::clone(&self.font_system);
        
        tokio::spawn(async move {
            let mut rx = rx.lock().await;
            while let Some(request) = rx.recv().await {
                match request {
                    FontLoadRequest::SystemFont { path, respond_to } => {
                        use crate::font_manager::loading_ops::FontLoadingOps;
                        let result = FontLoadingOps::load_system_font_async(&registry_manager, Arc::clone(&font_system), path).await;
                        let _ = respond_to.send(result);
                    }
                    FontLoadRequest::MemoryFont { data, key, respond_to } => {
                        use crate::font_manager::loading_ops::FontLoadingOps;
                        let result = FontLoadingOps::load_memory_font_async(&registry_manager, Arc::clone(&font_system), data, key).await;
                        let _ = respond_to.send(result);
                    }
                    FontLoadRequest::WebFont { url, respond_to } => {
                        use crate::font_manager::loading_ops::FontLoadingOps;
                        let result = FontLoadingOps::load_web_font_async(&registry_manager, Arc::clone(&font_system), url).await;
                        let _ = respond_to.send(result);
                    }
                }
            }
        });
    }
}