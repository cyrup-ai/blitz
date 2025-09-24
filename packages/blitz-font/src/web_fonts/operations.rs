use std::sync::{Arc, atomic::AtomicBool};
use std::time::{Duration, Instant};

use url::Url;

use crate::web_fonts::cache::CacheManager;
use crate::{FontError, FontKey, FontLoadStatus, WebFontEntry};

/// Background operations for web font loading
pub struct WebFontOperations;

impl WebFontOperations {
    /// Load a web font from URL asynchronously
    pub async fn load_font_async(
        client: &reqwest::Client,
        cache_manager: &CacheManager,
        _active_loads: &Arc<
            std::sync::atomic::AtomicPtr<std::collections::HashMap<Url, Arc<AtomicBool>>>,
        >,
        url: Url,
    ) -> Result<FontKey, FontError> {
        // Mark as loading
        let entry = WebFontEntry {
            url: url.to_string(),
            status: FontLoadStatus::Loading,
            data: None,
            load_start: Instant::now(),
            error: None,
            content_type: None,
            size: None,
            last_accessed: Instant::now(),
            access_count: 0,
        };

        cache_manager.insert(url.clone(), entry)?;

        // Download font data
        match client.get(url.clone()).send().await {
            Ok(response) => {
                match response.bytes().await {
                    Ok(data) => {
                        let data_vec = data.to_vec();

                        // Parse font and extract family name
                        match Self::create_font_key_from_data_sync(&data_vec, &url) {
                            Ok(font_key) => {
                                let loaded_entry = WebFontEntry {
                                    url: url.to_string(),
                                    status: FontLoadStatus::Loaded,
                                    data: Some(data_vec.clone()),
                                    load_start: Instant::now(),
                                    error: None,
                                    content_type: Some("font/woff2".to_string()),
                                    size: Some(data_vec.len() as u64),
                                    last_accessed: Instant::now(),
                                    access_count: 1,
                                };

                                let _ = cache_manager.insert(url, loaded_entry);
                                Ok(font_key)
                            }
                            Err(e) => {
                                let failed_entry = WebFontEntry {
                                    url: url.to_string(),
                                    status: FontLoadStatus::Failed,
                                    data: None,
                                    load_start: Instant::now(),
                                    error: Some(e.to_string()),
                                    content_type: None,
                                    size: None,
                                    last_accessed: Instant::now(),
                                    access_count: 0,
                                };

                                let _ = cache_manager.insert(url, failed_entry);
                                Err(e)
                            }
                        }
                    }
                    Err(e) => {
                        let error = FontError::NetworkError(e.to_string());
                        Self::mark_load_failed(cache_manager, url, &error).await;
                        Err(error)
                    }
                }
            }
            Err(e) => {
                let error = FontError::NetworkError(e.to_string());
                Self::mark_load_failed(cache_manager, url, &error).await;
                Err(error)
            }
        }
    }

    /// Mark a font load as failed
    async fn mark_load_failed(cache_manager: &CacheManager, url: Url, error: &FontError) {
        let failed_entry = WebFontEntry {
            url: url.to_string(),
            status: FontLoadStatus::Failed,
            data: None,
            load_start: Instant::now(),
            error: Some(error.to_string()),
            content_type: None,
            size: None,
            last_accessed: Instant::now(),
            access_count: 0,
        };

        let _ = cache_manager.insert(url, failed_entry);
    }

    /// Clear stale cache entries
    pub async fn clear_stale_entries_async(
        cache_manager: &CacheManager,
        _max_age: Duration,
    ) -> usize {
        let initial_size = cache_manager.size();
        let _ = cache_manager.cleanup_stale();
        let final_size = cache_manager.size();
        initial_size.saturating_sub(final_size)
    }

    /// Create font key from font data synchronously
    pub fn create_font_key_from_data_sync(data: &[u8], url: &Url) -> Result<FontKey, FontError> {
        use ttf_parser::Face;

        let face = Face::parse(data, 0)?;

        let family = face
            .names()
            .into_iter()
            .find(|name| name.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|name| name.to_string())
            .unwrap_or_else(|| {
                url.path_segments()
                    .and_then(|segments| segments.last())
                    .unwrap_or("Unknown")
                    .to_string()
            });

        // Extract weight, style, stretch from OS/2 table
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

    /// Implement exponential backoff waiting for font loads
    pub async fn wait_for_load_with_backoff(
        cache_manager: &CacheManager,
        url: &Url,
        max_wait: Duration,
    ) -> Result<FontKey, FontError> {
        let mut wait_time = Duration::from_millis(10);
        let start_time = Instant::now();

        loop {
            if start_time.elapsed() > max_wait {
                return Err(FontError::LoadTimeout);
            }

            tokio::time::sleep(wait_time).await;

            let cache = cache_manager.get_cache();
            if let Some(entry) = cache.get(url) {
                match entry.status {
                    FontLoadStatus::Loaded => {
                        if let Some(ref data) = entry.data {
                            return Self::create_font_key_from_data_sync(data, url);
                        }
                    }
                    FontLoadStatus::Failed => {
                        return Err(FontError::LoadFailed(
                            entry
                                .error
                                .clone()
                                .unwrap_or_else(|| "Load failed".to_string()),
                        ));
                    }
                    FontLoadStatus::Loading => {
                        // Continue waiting
                    }
                    _ => break,
                }
            }

            // Exponential backoff
            wait_time = (wait_time * 2).min(Duration::from_millis(1000));
        }

        Err(FontError::LoadTimeout)
    }
}
