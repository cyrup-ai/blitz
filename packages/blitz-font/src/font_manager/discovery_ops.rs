//! Font discovery and initialization operations
//!
//! This module handles system font discovery, initialization, and fallback chain setup
//! with optimized performance for zero-allocation hot paths.

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use glyphon::cosmyc_text::{Stretch, Style, Weight};

use crate::{
    FontError, FontKey, LoadedFont, SystemFont,
    font_manager::registry::RegistryManager,
    system_fonts::discovery::{DiscoveryConfig, SystemFontDiscovery},
};

/// Font discovery and initialization operations
pub struct DiscoveryOps;

impl DiscoveryOps {
    /// Initialize system fonts discovery with atomic state management (hot path optimized)
    #[inline(always)]
    pub fn initialize_system_fonts(
        registry_manager: &RegistryManager,
        font_system: &Arc<Mutex<glyphon::cosmyc_text::FontSystem>>,
        font_count: &AtomicUsize,
        system_fonts_loaded: &AtomicBool,
    ) -> Result<(), FontError> {
        if system_fonts_loaded.load(Ordering::Acquire) {
            return Ok(());
        }

        let discovered_fonts = Self::discover_system_fonts_internal()?;

        // Update registry with discovered fonts
        registry_manager.update_system_fonts(discovered_fonts.clone())?;

        // Convert SystemFont objects to LoadedFont objects and store in registry
        let mut loaded_count = 0;
        match font_system.lock() {
            Ok(mut font_system) => {
                for font in &discovered_fonts {
                    match Self::convert_system_font_to_loaded_font(font, &mut *font_system) {
                        Ok(loaded_font) => {
                            let font_key = font.to_font_key();
                            // LoadedFont is already registered with FontSystem, now add to registry
                            if let Err(e) = registry_manager.add_loaded_font(font_key, loaded_font)
                            {
                                log::error!(
                                    "Failed to register system font {}: {}",
                                    font.family,
                                    e
                                );
                            } else {
                                loaded_count += 1;
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to load system font {}: {}",
                                font.path.display(),
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                return Err(FontError::FontSystemError(format!(
                    "Failed to acquire FontSystem lock: {}",
                    e
                )));
            }
        }

        font_count.store(loaded_count, Ordering::Release);
        system_fonts_loaded.store(true, Ordering::Release);

        Ok(())
    }

    /// Internal system font discovery (platform-specific, cold path)
    #[inline]
    #[cold]
    pub fn discover_system_fonts_internal() -> Result<Vec<SystemFont>, FontError> {
        let discovery = SystemFontDiscovery::new(DiscoveryConfig::default());
        discovery.discover_all_fonts()
    }

    /// Setup default font fallback chains with comprehensive coverage (cold path)
    #[inline]
    #[cold]
    pub fn setup_default_fallbacks(registry_manager: &RegistryManager) -> Result<(), FontError> {
        let mut fallbacks = HashMap::new();

        // Serif fonts with comprehensive fallback chain
        fallbacks.insert(
            "serif".to_string(),
            vec![
                FontKey::new(
                    "Times New Roman".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Times".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Liberation Serif".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "serif".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
            ],
        );

        // Sans-serif fonts with comprehensive fallback chain
        fallbacks.insert(
            "sans-serif".to_string(),
            vec![
                FontKey::new(
                    "Arial".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Helvetica".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Liberation Sans".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "DejaVu Sans".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "sans-serif".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
            ],
        );

        // Monospace fonts with comprehensive fallback chain
        fallbacks.insert(
            "monospace".to_string(),
            vec![
                FontKey::new(
                    "Consolas".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Monaco".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Courier New".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Liberation Mono".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "DejaVu Sans Mono".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "monospace".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
            ],
        );

        // Cursive fonts with fallback chain
        fallbacks.insert(
            "cursive".to_string(),
            vec![
                FontKey::new(
                    "Comic Sans MS".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "Apple Chancery".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "cursive".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
            ],
        );

        // Fantasy fonts with fallback chain
        fallbacks.insert(
            "fantasy".to_string(),
            vec![
                FontKey::new(
                    "Impact".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
                FontKey::new(
                    "fantasy".to_string(),
                    Weight::NORMAL,
                    Style::Normal,
                    Stretch::Normal,
                ),
            ],
        );

        registry_manager.update_fallback_chains(fallbacks)
    }

    /// Register a custom fallback chain for a font family (cold path)
    #[inline]
    #[cold]
    pub fn register_fallback_chain(
        registry_manager: &RegistryManager,
        family: String,
        fallbacks: Vec<FontKey>,
    ) -> Result<(), FontError> {
        let registry = registry_manager.get_registry();
        let mut chains = (*registry.fallback_chains).clone();
        chains.insert(family, fallbacks);
        registry_manager.update_fallback_chains(chains)
    }

    /// Convert SystemFont to LoadedFont by reading and parsing font data (hot path optimized)
    #[inline(always)]
    fn convert_system_font_to_loaded_font(
        system_font: &SystemFont,
        font_system: &mut glyphon::cosmyc_text::FontSystem,
    ) -> Result<LoadedFont, FontError> {
        use ttf_parser::Face;

        use crate::{FontMetrics, FontSource};

        // Read font file data with size validation
        let data = std::fs::read(&system_font.path).map_err(|e| {
            FontError::IoError(format!(
                "Failed to read font file {:?}: {}",
                system_font.path, e
            ))
        })?;

        // Validate file size
        if data.len() < crate::constants::MIN_FONT_FILE_SIZE {
            return Err(FontError::InvalidFormat("Font file too small".to_string()));
        }

        if data.len() > crate::constants::MAX_FONT_FILE_SIZE {
            return Err(FontError::InvalidFormat("Font file too large".to_string()));
        }

        // Parse font with ttf-parser to extract metrics
        let face = Face::parse(&data, 0).map_err(|e| {
            FontError::ParseError(format!(
                "Failed to parse font {:?}: {:?}",
                system_font.path, e
            ))
        })?;

        // Extract font metrics
        let metrics = FontMetrics::from_face(&face);

        // Validate extracted metrics
        if !metrics.is_valid() {
            return Err(FontError::ParseError(
                "Invalid font metrics extracted".to_string(),
            ));
        }

        // Create LoadedFont with system source
        let font_key = system_font.to_font_key();
        let data_arc = Arc::from(data.into_boxed_slice());
        let mut loaded_font = LoadedFont::new(
            font_key,
            FontSource::System(system_font.path.clone()),
            data_arc,
            0, // face index
            metrics,
        );

        // Register with FontSystem atomically with LoadedFont creation
        loaded_font
            .register_with_font_system(font_system)
            .map_err(|e| {
                FontError::FontSystemError(format!(
                    "Failed to register font {} with FontSystem: {}",
                    system_font.family, e
                ))
            })?;

        Ok(loaded_font)
    }
}
