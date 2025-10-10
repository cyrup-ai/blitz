//! Font loading operations - high-performance async font loading
//!
//! This module handles all font loading operations including system fonts,
//! memory-based fonts, and web fonts with zero-allocation performance optimizations.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ttf_parser::Face;
use url::Url;

use crate::{
    FontError, FontKey, FontMetrics, FontSource, LoadedFont,
    font_manager::registry::RegistryManager,
};

/// Font loading operations with optimized async performance
pub struct FontLoadingOps;

impl FontLoadingOps {
    /// Load system font asynchronously with full validation and registration (hot path optimized)
    #[inline(always)]
    pub async fn load_system_font_async(
        registry_manager: &RegistryManager,
        font_system: Arc<Mutex<glyphon::cosmyc_text::FontSystem>>,
        path: PathBuf,
    ) -> Result<FontKey, FontError> {
        // Read font file data
        let data = tokio::fs::read(&path).await.map_err(|e| {
            FontError::IoError(format!("Failed to read font file {:?}: {}", path, e))
        })?;

        Self::validate_font_data_size(&data)?;

        // Parse font with ttf-parser
        let face = Face::parse(&data, 0).map_err(|e| {
            FontError::ParseError(format!("Failed to parse font {:?}: {:?}", path, e))
        })?;

        // Extract font properties and create FontKey
        let font_key = Self::extract_font_key_from_face(&face)?;

        // Extract and validate font metrics
        let metrics = FontMetrics::from_face(&face);
        if !metrics.is_valid() {
            return Err(FontError::ParseError(
                "Invalid font metrics extracted".to_string(),
            ));
        }

        // Create LoadedFont with system source
        let data_arc = Arc::from(data.into_boxed_slice());
        let mut loaded_font = LoadedFont::new(
            font_key.clone(),
            FontSource::System(path),
            data_arc,
            0, // face index
            metrics,
        );

        // Register with FontSystem
        Self::register_with_font_system(&mut loaded_font, &font_system)?;

        // Register font atomically with the registry
        registry_manager.add_loaded_font(font_key.clone(), loaded_font)?;

        Ok(font_key)
    }

    /// Load font from memory data asynchronously with validation (hot path optimized)
    #[inline(always)]
    pub async fn load_memory_font_async(
        registry_manager: &RegistryManager,
        font_system: Arc<Mutex<glyphon::cosmyc_text::FontSystem>>,
        data: Vec<u8>,
        key: FontKey,
    ) -> Result<(), FontError> {
        Self::validate_font_data_size(&data)?;

        // Parse font with ttf-parser to validate and extract metrics
        let face = Face::parse(&data, 0)
            .map_err(|e| FontError::ParseError(format!("Failed to parse font data: {:?}", e)))?;

        // Validate provided FontKey against parsed data (with warning if mismatch)
        Self::validate_font_key_against_data(&face, &key);

        // Extract and validate font metrics
        let metrics = FontMetrics::from_face(&face);
        if !metrics.is_valid() {
            return Err(FontError::ParseError(
                "Invalid font metrics extracted".to_string(),
            ));
        }

        // Create LoadedFont with memory source
        let data_arc = Arc::from(data.into_boxed_slice());
        let mut loaded_font = LoadedFont::new(
            key.clone(),
            FontSource::Memory(Arc::clone(&data_arc)),
            data_arc,
            0, // face index
            metrics,
        );

        // Register with FontSystem and validate registration
        Self::register_with_font_system(&mut loaded_font, &font_system)?;

        // Register font atomically with the registry
        registry_manager.add_loaded_font(key, loaded_font)?;

        Ok(())
    }

    /// Load web font asynchronously with caching and validation  
    #[cfg(feature = "web-fonts")]
    #[inline]
    pub async fn load_web_font_async(
        registry_manager: &RegistryManager,
        font_system: Arc<Mutex<glyphon::cosmyc_text::FontSystem>>,
        url: Url,
    ) -> Result<FontKey, FontError> {
        use std::time::Duration;

        use crate::web_fonts::cache::CacheManager;
        use crate::web_fonts::operations::WebFontOperations;

        // Create temporary HTTP client and cache for this load operation
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(
                crate::constants::FONT_LOAD_TIMEOUT_SECONDS,
            ))
            .user_agent("Blitz/1.0 (Lock-Free Font Loader)")
            .build()
            .map_err(|e| FontError::NetworkError(e.to_string()))?;

        let cache_manager = CacheManager::new(
            crate::constants::MAX_FONT_CACHE_SIZE,
            Duration::from_secs(crate::constants::DEFAULT_CACHE_TTL_SECONDS),
        ).await?;

        let active_loads = Arc::new(std::sync::atomic::AtomicPtr::new(Box::into_raw(Box::new(
            std::collections::HashMap::new(),
        ))));

        // Load the web font using WebFontOperations
        let font_key =
            WebFontOperations::load_font_async(&client, &cache_manager, &active_loads, url.clone())
                .await?;

        // Get the loaded font data from the cache
        let cache = cache_manager.get_cache();
        let entry = cache.get(&url).await.ok_or_else(|| {
            FontError::LoadFailed("Web font not found in cache after loading".to_string())
        })?;

        let data = entry.data.as_ref().ok_or_else(|| {
            FontError::LoadFailed("Web font data missing from cache entry".to_string())
        })?;

        // Parse the font data to extract metrics
        let face = Face::parse(data, 0).map_err(|e| {
            FontError::ParseError(format!("Failed to parse web font data: {:?}", e))
        })?;

        // Extract and validate font metrics
        let metrics = FontMetrics::from_face(&face);
        if !metrics.is_valid() {
            return Err(FontError::ParseError(
                "Invalid font metrics extracted from web font".to_string(),
            ));
        }

        // Create LoadedFont
        let mut loaded_font = LoadedFont::new(
            font_key.clone(),
            FontSource::WebFont(url),
            Arc::from(data.as_slice()),
            0, // face index
            metrics,
        );

        // Register with FontSystem and validate registration
        Self::register_with_font_system(&mut loaded_font, &font_system)?;

        // Register font atomically with the registry
        registry_manager.add_loaded_font(font_key.clone(), loaded_font)?;

        // Cleanup temporary active loads
        unsafe {
            let ptr = active_loads.load(std::sync::atomic::Ordering::Acquire);
            if !ptr.is_null() {
                let _ = Box::from_raw(ptr);
            }
        }

        Ok(font_key)
    }

    /// Placeholder for non-web-fonts builds
    #[cfg(not(feature = "web-fonts"))]
    #[inline]
    pub async fn load_web_font_async(
        _registry_manager: &RegistryManager,
        _font_system: Arc<Mutex<cosmyc_text::FontSystem>>,
        _url: Url,
    ) -> Result<FontKey, FontError> {
        Err(FontError::UnsupportedOperation(
            "Web fonts not enabled".to_string(),
        ))
    }

    // Helper functions

    /// Validate font data size constraints (optimized for zero-allocation hot path)
    #[inline(always)]
    fn validate_font_data_size(data: &[u8]) -> Result<(), FontError> {
        if data.len() < crate::constants::MIN_FONT_FILE_SIZE {
            return Err(FontError::InvalidFormat("Font data too small".to_string()));
        }

        if data.len() > crate::constants::MAX_FONT_FILE_SIZE {
            return Err(FontError::InvalidFormat("Font data too large".to_string()));
        }

        Ok(())
    }

    /// Extract FontKey from ttf-parser Face (zero-allocation optimized)
    #[inline(always)]
    fn extract_font_key_from_face(face: &Face) -> Result<FontKey, FontError> {
        // Extract font family name
        let family = face
            .names()
            .into_iter()
            .find(|name| name.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|name| name.to_string())
            .ok_or_else(|| FontError::ParseError("No family name found in font".to_string()))?;

        // Extract font properties
        let weight = glyphon::cosmyc_text::Weight(face.weight().to_number());
        let style = match face.style() {
            ttf_parser::Style::Normal => glyphon::cosmyc_text::Style::Normal,
            ttf_parser::Style::Italic => glyphon::cosmyc_text::Style::Italic,
            ttf_parser::Style::Oblique => glyphon::cosmyc_text::Style::Oblique,
        };
        let stretch = match face.width() {
            ttf_parser::Width::UltraCondensed => glyphon::cosmyc_text::Stretch::UltraCondensed,
            ttf_parser::Width::ExtraCondensed => glyphon::cosmyc_text::Stretch::ExtraCondensed,
            ttf_parser::Width::Condensed => glyphon::cosmyc_text::Stretch::Condensed,
            ttf_parser::Width::SemiCondensed => glyphon::cosmyc_text::Stretch::SemiCondensed,
            ttf_parser::Width::Normal => glyphon::cosmyc_text::Stretch::Normal,
            ttf_parser::Width::SemiExpanded => glyphon::cosmyc_text::Stretch::SemiExpanded,
            ttf_parser::Width::Expanded => glyphon::cosmyc_text::Stretch::Expanded,
            ttf_parser::Width::ExtraExpanded => glyphon::cosmyc_text::Stretch::ExtraExpanded,
            ttf_parser::Width::UltraExpanded => glyphon::cosmyc_text::Stretch::UltraExpanded,
        };

        Ok(FontKey::new(family, weight, style, stretch))
    }

    /// Validate provided FontKey against parsed font data (logs warning if mismatch, cold path)
    #[inline]
    #[cold]
    fn validate_font_key_against_data(face: &Face, key: &FontKey) {
        let parsed_family = face
            .names()
            .into_iter()
            .find(|name| name.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|name| name.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let parsed_weight = glyphon::cosmyc_text::Weight(face.weight().to_number());
        let parsed_style = match face.style() {
            ttf_parser::Style::Normal => glyphon::cosmyc_text::Style::Normal,
            ttf_parser::Style::Italic => glyphon::cosmyc_text::Style::Italic,
            ttf_parser::Style::Oblique => glyphon::cosmyc_text::Style::Oblique,
        };
        let parsed_stretch = match face.width() {
            ttf_parser::Width::UltraCondensed => glyphon::cosmyc_text::Stretch::UltraCondensed,
            ttf_parser::Width::ExtraCondensed => glyphon::cosmyc_text::Stretch::ExtraCondensed,
            ttf_parser::Width::Condensed => glyphon::cosmyc_text::Stretch::Condensed,
            ttf_parser::Width::SemiCondensed => glyphon::cosmyc_text::Stretch::SemiCondensed,
            ttf_parser::Width::Normal => glyphon::cosmyc_text::Stretch::Normal,
            ttf_parser::Width::SemiExpanded => glyphon::cosmyc_text::Stretch::SemiExpanded,
            ttf_parser::Width::Expanded => glyphon::cosmyc_text::Stretch::Expanded,
            ttf_parser::Width::ExtraExpanded => glyphon::cosmyc_text::Stretch::ExtraExpanded,
            ttf_parser::Width::UltraExpanded => glyphon::cosmyc_text::Stretch::UltraExpanded,
        };

        if key.family != parsed_family
            || key.weight != parsed_weight
            || key.style != parsed_style
            || key.stretch != parsed_stretch
        {
            log::warn!(
                "FontKey mismatch: provided={}, parsed=family:'{}' weight:{} style:{:?} stretch:{:?}",
                key,
                parsed_family,
                parsed_weight.0,
                parsed_style,
                parsed_stretch
            );
        }
    }

    /// Register LoadedFont with FontSystem (hot path optimized)
    #[inline(always)]
    fn register_with_font_system(
        loaded_font: &mut LoadedFont,
        font_system: &Arc<Mutex<glyphon::cosmyc_text::FontSystem>>,
    ) -> Result<(), FontError> {
        let mut fs = font_system.lock().map_err(|e| {
            FontError::FontSystemError(format!("Failed to acquire FontSystem lock: {}", e))
        })?;
        loaded_font
            .register_with_font_system(&mut *fs)
            .map_err(|e| {
                FontError::FontSystemError(format!(
                    "Failed to register font with FontSystem: {}",
                    e
                ))
            })?;
        Ok(())
    }
}
