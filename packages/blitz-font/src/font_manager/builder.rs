use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::{FontError, FontKey, FontManager};

/// Configuration builder for FontManager with fluent API
#[derive(Debug, Clone)]
pub struct FontManagerBuilder {
    pub discover_system_fonts: bool,
    pub setup_fallbacks: bool,
    pub web_font_enabled: bool,
    pub font_directories: Vec<PathBuf>,
    pub custom_fallbacks: HashMap<String, Vec<FontKey>>,
    pub max_cache_size: usize,
    pub cache_ttl: Duration,
}

impl FontManagerBuilder {
    /// Create a new FontManagerBuilder with optimal defaults
    pub fn new() -> Self {
        Self {
            discover_system_fonts: true,
            setup_fallbacks: true,
            web_font_enabled: cfg!(feature = "web-fonts"),
            font_directories: Vec::new(),
            custom_fallbacks: HashMap::new(),
            max_cache_size: 1024,
            cache_ttl: Duration::from_secs(3600), // 1 hour
        }
    }

    /// Disable automatic system font discovery
    #[inline]
    pub fn disable_system_fonts(mut self) -> Self {
        self.discover_system_fonts = false;
        self
    }

    /// Disable default font fallback chains
    #[inline]
    pub fn disable_fallbacks(mut self) -> Self {
        self.setup_fallbacks = false;
        self
    }

    /// Add a custom font directory to scan
    #[inline]
    pub fn add_font_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.font_directories.push(path.into());
        self
    }

    /// Add a custom fallback chain for a font family
    #[inline]
    pub fn add_fallback_chain(mut self, family: String, fallbacks: Vec<FontKey>) -> Self {
        self.custom_fallbacks.insert(family, fallbacks);
        self
    }

    /// Set maximum cache size (number of fonts)
    #[inline]
    pub fn with_max_cache_size(mut self, size: usize) -> Self {
        self.max_cache_size = size;
        self
    }

    /// Set cache time-to-live
    #[inline]
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Enable web font loading (requires "web-fonts" feature)
    #[inline]
    #[cfg(feature = "web-fonts")]
    pub fn enable_web_fonts(mut self) -> Self {
        self.web_font_enabled = true;
        self
    }

    /// Disable web font loading
    #[inline]
    pub fn disable_web_fonts(mut self) -> Self {
        self.web_font_enabled = false;
        self
    }

    /// Add multiple font directories at once
    #[inline]
    pub fn with_font_directories<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.font_directories
            .extend(paths.into_iter().map(Into::into));
        self
    }

    /// Configure cache settings in one call
    #[inline]
    pub fn with_cache_config(mut self, size: usize, ttl: Duration) -> Self {
        self.max_cache_size = size;
        self.cache_ttl = ttl;
        self
    }

    /// Add fallback chains from a HashMap
    #[inline]
    pub fn with_fallback_chains(mut self, chains: HashMap<String, Vec<FontKey>>) -> Self {
        self.custom_fallbacks.extend(chains);
        self
    }

    /// Create a minimal configuration (no system fonts, no fallbacks)
    pub fn minimal() -> Self {
        Self {
            discover_system_fonts: false,
            setup_fallbacks: false,
            web_font_enabled: false,
            font_directories: Vec::new(),
            custom_fallbacks: HashMap::new(),
            max_cache_size: 64,
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create a performance-optimized configuration
    pub fn performance_optimized() -> Self {
        Self {
            discover_system_fonts: true,
            setup_fallbacks: true,
            web_font_enabled: cfg!(feature = "web-fonts"),
            font_directories: Vec::new(),
            custom_fallbacks: HashMap::new(),
            max_cache_size: 2048,
            cache_ttl: Duration::from_secs(7200), // 2 hours
        }
    }

    /// Validate configuration before building
    pub fn validate(&self) -> Result<(), FontError> {
        if self.max_cache_size == 0 {
            return Err(FontError::ConfigError(
                "Cache size cannot be zero".to_string(),
            ));
        }

        if self.cache_ttl.is_zero() {
            return Err(FontError::ConfigError(
                "Cache TTL cannot be zero".to_string(),
            ));
        }

        // Validate font directories exist
        for dir in &self.font_directories {
            if !dir.exists() {
                return Err(FontError::ConfigError(format!(
                    "Font directory does not exist: {}",
                    dir.display()
                )));
            }

            if !dir.is_dir() {
                return Err(FontError::ConfigError(format!(
                    "Font directory path is not a directory: {}",
                    dir.display()
                )));
            }
        }

        Ok(())
    }

    /// Build the FontManager with the specified configuration
    pub async fn build(self) -> Result<FontManager, FontError> {
        self.validate()?;
        FontManager::with_config(self).await
    }
}

impl Default for FontManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
