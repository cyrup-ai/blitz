//! Enhanced font system functionality
//!
//! This module provides enhanced font system capabilities with thread-local
//! initialization and improved error handling.

use std::sync::Arc;

use cosmyc_text::{Attrs, Font, FontSystem, PlatformFallback};

/// Enhanced font system wrapper
#[derive(Debug)]
pub struct EnhancedFontSystem {
    inner: FontSystem,
    embedded_fallback_id: cosmyc_text::fontdb::ID,
}

impl EnhancedFontSystem {
    /// Create new enhanced font system with thread-local initialization
    pub fn new() -> Self {
        // Ensure thread-local FontSystem is initialized WITH embedded font
        let _ = crate::measurement::thread_local::with_font_system(|font_system| {
            // Thread-local FontSystem is now initialized with embedded font
            crate::embedded_fallback::ensure_embedded_fallback(font_system)
        });

        let mut font_system = FontSystem::new();

        // CRITICAL: Load embedded fallback font to guarantee font_id validity
        let embedded_fallback_id =
            crate::embedded_fallback::load_embedded_fallback(font_system.db_mut());

        log::debug!(
            "Loaded embedded fallback font with ID: {:?}",
            embedded_fallback_id
        );

        Self {
            inner: font_system,
            embedded_fallback_id,
        }
    }

    /// Create from existing FontSystem (for compatibility)
    pub fn from_font_system(source_font_system: &FontSystem) -> Self {
        // 1. Extract font database information
        let source_db = source_font_system.db();
        let locale = source_font_system.locale().to_string();

        // 2. Create new database with same configuration
        let mut new_db = cosmyc_text::fontdb::Database::new();

        // 3. Note: Font family configurations cannot be extracted from source database
        // as fontdb::Database only provides setters, not getters for family configurations

        // 4. Copy all loaded fonts from source database
        for face_info in source_db.faces() {
            // Extract font data and re-add to new database
            source_db.with_face_data(face_info.id, |font_data, _face_index| {
                let source = cosmyc_text::fontdb::Source::Binary(Arc::new(font_data.to_vec()));
                new_db.load_font_source(source);
            });
        }

        // 5. Add embedded fallback to new database
        let embedded_fallback_id = crate::embedded_fallback::load_embedded_fallback(&mut new_db);

        // 6. Create enhanced font system with transferred data
        Self {
            inner: FontSystem::new_with_locale_and_db_and_fallback(
                locale,
                new_db,
                PlatformFallback,
            ),
            embedded_fallback_id,
        }
    }

    /// Create with custom fonts
    pub fn with_fonts(fonts: impl IntoIterator<Item = cosmyc_text::fontdb::Source>) -> Self {
        // Ensure thread-local FontSystem is initialized WITH embedded font
        let _ = crate::measurement::thread_local::with_font_system(|font_system| {
            crate::embedded_fallback::ensure_embedded_fallback(font_system)
        });

        let mut font_system = FontSystem::new_with_fonts(fonts);
        let embedded_fallback_id =
            crate::embedded_fallback::load_embedded_fallback(font_system.db_mut());

        Self {
            inner: font_system,
            embedded_fallback_id,
        }
    }

    /// Get reference to inner font system
    pub fn inner(&self) -> &FontSystem {
        &self.inner
    }

    /// Get mutable reference to inner font system
    pub fn inner_mut(&mut self) -> &mut FontSystem {
        &mut self.inner
    }

    /// Get font with enhanced error handling
    pub fn get_font_safe(
        &mut self,
        id: cosmyc_text::fontdb::ID,
        weight: cosmyc_text::fontdb::Weight,
    ) -> Option<Arc<Font>> {
        self.inner.get_font(id, weight)
    }

    /// Get font matches with caching
    pub fn get_font_matches_cached(
        &mut self,
        attrs: &Attrs,
    ) -> Arc<Vec<cosmyc_text::FontMatchKey>> {
        self.inner.get_font_matches(attrs)
    }

    /// Query font database
    pub fn query_font(
        &self,
        query: &cosmyc_text::fontdb::Query,
    ) -> Option<cosmyc_text::fontdb::ID> {
        self.inner.db().query(query)
    }

    /// Get font face information
    pub fn get_face_info(
        &self,
        id: cosmyc_text::fontdb::ID,
    ) -> Option<&cosmyc_text::fontdb::FaceInfo> {
        self.inner.db().face(id)
    }

    /// List all available font families
    pub fn list_font_families(&self) -> Vec<String> {
        self.inner
            .db()
            .faces()
            .filter_map(|face_info| face_info.families.first().map(|(name, _)| name.clone()))
            .collect()
    }

    /// Get the font ID of the guaranteed embedded fallback font
    pub fn embedded_fallback_id(&self) -> cosmyc_text::fontdb::ID {
        self.embedded_fallback_id
    }

    /// Get font data with guaranteed success using embedded fallback
    ///
    /// This method NEVER returns None. If the requested font_id is not found,
    /// it automatically falls back to the embedded fallback font.
    ///
    /// Returns (font_data, face_index) tuple.
    pub fn get_font_data_guaranteed(
        &self,
        font_id: cosmyc_text::fontdb::ID,
    ) -> (Vec<u8>, u32) {
        self.inner
            .db()
            .with_face_data(font_id, |data, face_index| (data.to_vec(), face_index))
            .unwrap_or_else(|| {
                log::warn!(
                    "Font ID {:?} not found in database, using embedded fallback (ID: {:?})",
                    font_id,
                    self.embedded_fallback_id
                );

                // Fall back to embedded font (guaranteed to exist)
                self.inner
                    .db()
                    .with_face_data(self.embedded_fallback_id, |data, face_index| {
                        (data.to_vec(), face_index)
                    })
                    .expect("BUG: Embedded fallback font must always be available in database")
            })
    }
}

impl Default for EnhancedFontSystem {
    fn default() -> Self {
        Self::new()
    }
}
