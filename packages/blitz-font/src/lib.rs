//! Comprehensive lock-free font management system for Blitz browser engine
//!
//! This crate provides a blazing-fast, zero-allocation font management system that handles:
//! - System font discovery and loading
//! - Web font loading and caching  
//! - Font fallback chain management
//! - Integration with blitz-text for text rendering
//!
//! # Architecture
//!
//! The system is designed to be completely lock-free for maximum performance:

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unexpected_cfgs)]
//! - Uses atomic operations and immutable data structures
//! - Employs message passing via channels for async operations
//! - Zero-allocation hot paths with efficient caching
//!
//! # Features
//!
//! - **Lock-free design**: No RwLock/Mutex for blazing-fast concurrent access
//! - **System fonts**: Automatic discovery of fonts on Windows, macOS, and Linux
//! - **Web fonts**: Async loading and caching of remote fonts with WOFF/WOFF2 support
//! - **Font metrics**: Comprehensive font metrics extraction for layout calculations
//! - **Fallback chains**: Intelligent font fallback system for missing glyphs
//! - **Performance optimized**: Zero allocation in hot paths, atomic operations only
//!
//! # Example
//!
//! ```rust,no_run
//! use blitz_font::{FontManager, FontKey};
//! use blitz_text::{Weight, Style, Stretch};
//!
//! # async fn example() -> Result<(), blitz_font::FontError> {
//! // Initialize logging (optional, for development)
//! let _ = env_logger::try_init();
//!
//! // Create a font manager
//! let font_manager = FontManager::new()?;
//!
//! // Find a font
//! let font_key = FontKey::new(
//!     "Arial".to_string(),
//!     Weight::NORMAL,
//!     Style::Normal,
//!     Stretch::Normal
//! );
//!
//! if let Some(matched_font) = font_manager.find_best_font(&font_key).await {
//!     log::info!("Found font: {}", matched_font);
//! }
//!
//! // Load a web font
//! let url = "https://fonts.gstatic.com/s/roboto/v30/KFOmCnqEu92Fr1Mu4mxK.woff2".parse()?;
//! let web_font_key = font_manager.load_web_font(url).await?;
//! log::info!("Loaded web font: {}", web_font_key);
//! # Ok(())
//! # }
//! ```

// Re-export blitz-text types for convenience
pub use blitz_text::{Family, Stretch, Style, Weight};

mod error;
mod font_manager;
mod loaded_font;
mod metrics;
mod system_font;
mod system_fonts;
mod types;
mod web_font_entry;

// Conditionally compile web font support
#[cfg(feature = "web-fonts")]
mod web_fonts;

// Public API exports
pub use error::{
    ContextualizedFontError, FontError, FontErrorContext, FontErrorSeverity, FontResult,
    FontWarning,
};
pub use font_manager::{FontManager, FontManagerBuilder};
pub use loaded_font::{FontFormat, FontUsageReport, LoadedFont};
pub use metrics::{FontLayoutMetrics, FontMetrics};
pub use system_font::{FontCapabilities, SystemFont, WritingScript};
pub use types::{FontKey, FontLoadStatus, FontSource};
pub use web_font_entry::{WebFontCacheStats, WebFontEntry, WebFontStatusReport};
#[cfg(feature = "web-fonts")]
pub use web_fonts::{WebFontCache, WebFontLoader};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Utility functions for font management
pub mod utils {
    use super::*;

    /// Extract font family name from a font file path
    #[inline]
    pub fn extract_family_from_path(path: &std::path::Path) -> Result<String, FontError> {
        let data = std::fs::read(path)?;
        extract_family_from_data(&data)
    }

    /// Extract font family name from font data
    pub fn extract_family_from_data(data: &[u8]) -> Result<String, FontError> {
        use ttf_parser::Face;

        let face = Face::parse(data, 0)?;

        let family = face
            .names()
            .into_iter()
            .find(|name| name.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|name| name.to_string())
            .ok_or_else(|| FontError::ParseError("No family name found".to_string()))?;

        Ok(family)
    }

    /// Check if a font file supports a specific Unicode codepoint
    #[inline]
    pub fn font_supports_codepoint(
        path: &std::path::Path,
        codepoint: u32,
    ) -> Result<bool, FontError> {
        let data = std::fs::read(path)?;
        font_data_supports_codepoint(&data, codepoint)
    }

    /// Check if font data supports a specific Unicode codepoint
    pub fn font_data_supports_codepoint(data: &[u8], codepoint: u32) -> Result<bool, FontError> {
        use ttf_parser::Face;

        let face = Face::parse(data, 0)?;
        let char = char::from_u32(codepoint)
            .ok_or_else(|| FontError::InvalidFormat("Invalid Unicode codepoint".to_string()))?;
        Ok(face.glyph_index(char).is_some())
    }

    /// Get font format from file extension
    #[inline]
    pub fn get_font_format_from_extension(path: &std::path::Path) -> Option<FontFormat> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "ttf" => FontFormat::TrueType,
                "otf" => FontFormat::OpenType,
                "woff" => FontFormat::WOFF,
                "woff2" => FontFormat::WOFF2,
                "ttc" => FontFormat::TrueTypeCollection,
                "otc" => FontFormat::OpenTypeCollection,
                _ => FontFormat::Unknown,
            })
    }

    /// Calculate font matching score for font selection
    #[inline]
    pub fn calculate_font_match_score(candidate: &FontKey, requested: &FontKey) -> u32 {
        let mut score = 0u32;

        // Weight difference (most important)
        let weight_diff = (candidate.weight.0 as i32 - requested.weight.0 as i32).abs();
        score += weight_diff as u32;

        // Style mismatch penalty
        if candidate.style != requested.style {
            score += 100;
        }

        // Stretch mismatch penalty
        if candidate.stretch != requested.stretch {
            score += 50;
        }

        score
    }

    /// Check if two font families are compatible
    #[inline]
    pub fn are_font_families_compatible(font_family: &str, requested_family: &str) -> bool {
        let font_base = font_family.split_whitespace().next().unwrap_or(font_family);
        let requested_base = requested_family
            .split_whitespace()
            .next()
            .unwrap_or(requested_family);

        font_base.eq_ignore_ascii_case(requested_base)
    }
}

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{
        FontError, FontErrorSeverity, FontKey, FontManager, FontManagerBuilder, FontMetrics,
        FontResult, FontSource, FontWarning, LoadedFont, Stretch, Style, SystemFont, Weight,
    };
    #[cfg(feature = "web-fonts")]
    pub use super::{FontLoadStatus, WebFontEntry, WebFontLoader};
}

/// Performance-critical constants
pub mod constants {
    /// Maximum number of fonts to keep in memory cache
    pub const MAX_FONT_CACHE_SIZE: usize = 1024;

    /// Default cache TTL for web fonts (1 hour)
    pub const DEFAULT_CACHE_TTL_SECONDS: u64 = 3600;

    /// Maximum concurrent font loading operations
    pub const MAX_CONCURRENT_LOADS: usize = 32;

    /// Font loading timeout in seconds
    pub const FONT_LOAD_TIMEOUT_SECONDS: u64 = 30;

    /// Minimum font file size (4 bytes for signature)
    pub const MIN_FONT_FILE_SIZE: usize = 4;

    /// Maximum font file size (50MB)
    pub const MAX_FONT_FILE_SIZE: usize = 50 * 1024 * 1024;
}
