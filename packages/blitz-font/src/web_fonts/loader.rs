use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use url::Url;

use crate::web_fonts::cache::CacheManager;
use crate::{FontError, FontKey, FontLoadStatus, WebFontCacheStats, WebFontEntry};

#[derive(Debug, Clone)]
pub struct WebFontLoaderConfig {
    pub max_cache_size: usize,
    pub cache_ttl: Duration,
    pub load_timeout: Duration,
    pub user_agent: String,
}

impl Default for WebFontLoaderConfig {
    fn default() -> Self {
        Self {
            max_cache_size: crate::constants::MAX_FONT_CACHE_SIZE,
            cache_ttl: Duration::from_secs(crate::constants::DEFAULT_CACHE_TTL_SECONDS),
            load_timeout: Duration::from_secs(30),
            user_agent: "Blitz/1.0 (Lock-Free Font Loader)".to_string(),
        }
    }
}

#[derive(Debug)]
enum WebFontOperation {
    LoadFont {
        url: Url,
        respond_to: tokio::sync::oneshot::Sender<Result<FontKey, FontError>>,
    },
    ClearStaleEntries {
        max_age: Duration,
        respond_to: tokio::sync::oneshot::Sender<usize>,
    },
    GetStats {
        respond_to: tokio::sync::oneshot::Sender<WebFontCacheStats>,
    },
}

/// Lock-free web font loader for async loading and caching of remote fonts
pub struct WebFontLoader {
    cache_manager: CacheManager,
    client: reqwest::Client,
    active_loads: Arc<std::sync::atomic::AtomicPtr<HashMap<Url, Arc<AtomicBool>>>>,
    load_timeout: Duration,

    // Channel for async operations
    operation_tx: tokio::sync::mpsc::UnboundedSender<WebFontOperation>,
    operation_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<WebFontOperation>>>,
}

impl WebFontLoader {
    /// Create a new WebFontLoader with default configuration
    ///
    /// # Errors
    /// Returns `FontError::CacheInitializationError` if cache creation fails
    /// Returns `FontError::NetworkError` if HTTP client creation fails
    pub fn new() -> Result<Self, FontError> {
        Self::with_config(WebFontLoaderConfig::default())
    }

    /// Create a new WebFontLoader with custom configuration
    ///
    /// # Errors
    /// Returns `FontError::CacheInitializationError` if cache creation fails
    /// Returns `FontError::NetworkError` if HTTP client creation fails
    pub fn with_config(config: WebFontLoaderConfig) -> Result<Self, FontError> {
        let cache_manager =
            CacheManager::new(config.max_cache_size, config.cache_ttl).map_err(|e| {
                FontError::CacheInitializationError(format!(
                    "Failed to initialize cache manager: {}",
                    e
                ))
            })?;

        let client = reqwest::Client::builder()
            .timeout(config.load_timeout)
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| FontError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let initial_loads = HashMap::new();
        let (operation_tx, operation_rx) = tokio::sync::mpsc::unbounded_channel();

        let loader = Self {
            cache_manager,
            client,
            active_loads: Arc::new(std::sync::atomic::AtomicPtr::new(Box::into_raw(Box::new(
                initial_loads,
            )))),
            load_timeout: config.load_timeout,
            operation_tx,
            operation_rx: Arc::new(tokio::sync::Mutex::new(operation_rx)),
        };

        // Start background operation processor
        loader.start_operation_processor();

        Ok(loader)
    }

    /// Create a WebFontLoader with minimal safe defaults for testing
    ///
    /// Uses in-memory only cache and basic HTTP client
    /// Should only be used in test environments
    pub fn minimal() -> Self {
        // This is the only place where we allow unwrap, but only for testing
        // and with guaranteed-safe minimal configuration
        let cache_manager = CacheManager::new(100, Duration::from_secs(60))
            .expect("Minimal cache configuration should never fail");

        let client = reqwest::Client::new(); // Basic client without custom config

        let initial_loads = HashMap::new();
        let (operation_tx, operation_rx) = tokio::sync::mpsc::unbounded_channel();

        let loader = WebFontLoader {
            cache_manager,
            client,
            active_loads: Arc::new(std::sync::atomic::AtomicPtr::new(Box::into_raw(Box::new(
                initial_loads,
            )))),
            load_timeout: Duration::from_secs(30),
            operation_tx,
            operation_rx: Arc::new(tokio::sync::Mutex::new(operation_rx)),
        };

        // Start background operation processor
        loader.start_operation_processor();

        loader
    }

    /// Load a web font from URL, with caching and concurrent request deduplication
    pub async fn load_font(&self, url: Url) -> Result<FontKey, FontError> {
        // Check cache first
        let cache = self.cache_manager.get_cache();
        if let Some(entry) = cache.get(&url) {
            match entry.status {
                FontLoadStatus::Loaded => {
                    if let Some(ref data) = entry.data {
                        return self.create_font_key_from_data(data, &url);
                    }
                }
                FontLoadStatus::Loading => {
                    return self.wait_for_load(&url).await;
                }
                FontLoadStatus::Failed => {
                    if entry.load_start.elapsed() < Duration::from_secs(300) {
                        return Err(FontError::LoadFailed(
                            entry
                                .error
                                .clone()
                                .unwrap_or_else(|| "Load failed".to_string()),
                        ));
                    }
                    // Retry after timeout
                }
                _ => {}
            }
        }

        // Send load request to background processor
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.operation_tx
            .send(WebFontOperation::LoadFont {
                url,
                respond_to: tx,
            })
            .map_err(|_| FontError::FontSystemError("Operation channel closed".to_string()))?;

        rx.await
            .map_err(|_| FontError::FontSystemError("Load operation response lost".to_string()))?
    }

    /// Wait for concurrent load to complete
    async fn wait_for_load(&self, url: &Url) -> Result<FontKey, FontError> {
        crate::web_fonts::operations::WebFontOperations::wait_for_load_with_backoff(
            &self.cache_manager,
            url,
            Duration::from_secs(30),
        )
        .await
    }

    /// Start background operation processor
    fn start_operation_processor(&self) {
        let operation_rx = Arc::clone(&self.operation_rx);
        let client = self.client.clone();
        let cache_manager = self.cache_manager.clone();
        let active_loads = Arc::clone(&self.active_loads);

        tokio::spawn(async move {
            let mut rx = operation_rx.lock().await;
            while let Some(operation) = rx.recv().await {
                match operation {
                    WebFontOperation::LoadFont { url, respond_to } => {
                        let result =
                            Self::load_font_async(&client, &cache_manager, &active_loads, url)
                                .await;
                        let _ = respond_to.send(result);
                    }
                    WebFontOperation::ClearStaleEntries {
                        max_age,
                        respond_to,
                    } => {
                        let result = Self::clear_stale_entries_async(&cache_manager, max_age).await;
                        let _ = respond_to.send(result);
                    }
                    WebFontOperation::GetStats { respond_to } => {
                        let stats = cache_manager.get_stats();
                        let _ = respond_to.send(stats);
                    }
                }
            }
        });
    }

    async fn load_font_async(
        client: &reqwest::Client,
        cache_manager: &CacheManager,
        active_loads: &Arc<std::sync::atomic::AtomicPtr<HashMap<Url, Arc<AtomicBool>>>>,
        url: Url,
    ) -> Result<FontKey, FontError> {
        crate::web_fonts::operations::WebFontOperations::load_font_async(
            client,
            cache_manager,
            active_loads,
            url,
        )
        .await
    }

    async fn clear_stale_entries_async(cache_manager: &CacheManager, max_age: Duration) -> usize {
        crate::web_fonts::operations::WebFontOperations::clear_stale_entries_async(
            cache_manager,
            max_age,
        )
        .await
    }

    fn create_font_key_from_data(&self, data: &[u8], url: &Url) -> Result<FontKey, FontError> {
        crate::web_fonts::operations::WebFontOperations::create_font_key_from_data_sync(data, url)
    }

    /// Get cached font entry
    pub fn get_cached_entry(&self, url: &Url) -> Result<Option<WebFontEntry>, FontError> {
        let cache = self.cache_manager.get_cache();
        Ok(cache.get(url))
    }

    /// Preload a font without blocking
    pub fn preload_font(&self, url: Url) -> tokio::task::JoinHandle<Result<FontKey, FontError>> {
        let loader = self.clone();
        tokio::spawn(async move { loader.load_font(url).await })
    }

    /// Get current cache size
    pub fn cache_size(&self) -> usize {
        self.cache_manager.size()
    }

    /// Check if cache is near capacity  
    pub fn is_cache_near_capacity(&self) -> bool {
        self.cache_manager.is_near_capacity()
    }
}

impl Clone for WebFontLoader {
    fn clone(&self) -> Self {
        Self {
            cache_manager: self.cache_manager.clone(),
            client: self.client.clone(),
            active_loads: Arc::clone(&self.active_loads),
            load_timeout: self.load_timeout,
            operation_tx: self.operation_tx.clone(),
            operation_rx: Arc::clone(&self.operation_rx),
        }
    }
}

impl Drop for WebFontLoader {
    fn drop(&mut self) {
        // Channel cleanup is automatic
        let ptr = self.active_loads.load(Ordering::Acquire);
        if !ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
    }
}
