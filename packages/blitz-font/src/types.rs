use std::path::PathBuf;
use std::sync::Arc;

use glyphon::cosmyc_text::{Stretch, Style, Weight};
use serde::{Deserialize, Serialize};
use url::Url;

/// Unique identifier for a font combining family, weight, style, and stretch
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FontKey {
    pub family: String,
    pub weight: Weight,
    pub style: Style,
    pub stretch: Stretch,
}

impl FontKey {
    /// Create a new FontKey
    pub fn new(family: String, weight: Weight, style: Style, stretch: Stretch) -> Self {
        Self {
            family,
            weight,
            style,
            stretch,
        }
    }

    /// Create a FontKey for a normal variant of a font family
    pub fn normal(family: String) -> Self {
        Self {
            family,
            weight: Weight::NORMAL,
            style: Style::Normal,
            stretch: Stretch::Normal,
        }
    }

    /// Create a FontKey for a bold variant
    pub fn bold(family: String) -> Self {
        Self {
            family,
            weight: Weight::BOLD,
            style: Style::Normal,
            stretch: Stretch::Normal,
        }
    }

    /// Create a FontKey for an italic variant
    pub fn italic(family: String) -> Self {
        Self {
            family,
            weight: Weight::NORMAL,
            style: Style::Italic,
            stretch: Stretch::Normal,
        }
    }

    /// Create a FontKey for a bold italic variant
    pub fn bold_italic(family: String) -> Self {
        Self {
            family,
            weight: Weight::BOLD,
            style: Style::Italic,
            stretch: Stretch::Normal,
        }
    }
}

impl std::fmt::Display for FontKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {:?}",
            self.family,
            self.weight.0,
            match self.style {
                Style::Normal => "normal",
                Style::Italic => "italic",
                Style::Oblique => "oblique",
            },
            self.stretch
        )
    }
}

/// Source location of a font
#[derive(Debug, Clone)]
pub enum FontSource {
    /// System font from file system
    System(PathBuf),
    /// Web font from URL
    WebFont(Url),
    /// Embedded font data
    Embedded(&'static [u8]),
    /// Font data loaded into memory
    Memory(Arc<[u8]>),
}

impl FontSource {
    /// Get a display name for the font source
    pub fn display_name(&self) -> String {
        match self {
            FontSource::System(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("system font")
                .to_string(),
            FontSource::WebFont(url) => url
                .path_segments()
                .and_then(|segments| segments.last())
                .unwrap_or(url.as_str())
                .to_string(),
            FontSource::Embedded(_) => "embedded".to_string(),
            FontSource::Memory(_) => "memory".to_string(),
        }
    }

    /// Check if this is a local font source
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            FontSource::System(_) | FontSource::Embedded(_) | FontSource::Memory(_)
        )
    }

    /// Check if this is a remote font source
    pub fn is_remote(&self) -> bool {
        matches!(self, FontSource::WebFont(_))
    }
}

/// Font loading status enumeration
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum FontLoadStatus {
    /// Font loading has not started
    #[default]
    NotStarted,
    /// Font is currently being loaded
    Loading,
    /// Font has been successfully loaded
    Loaded,
    /// Font loading failed
    Failed,
    /// Font is cached and ready for use
    Cached,
}

impl std::fmt::Display for FontLoadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontLoadStatus::NotStarted => write!(f, "not started"),
            FontLoadStatus::Loading => write!(f, "loading"),
            FontLoadStatus::Loaded => write!(f, "loaded"),
            FontLoadStatus::Failed => write!(f, "failed"),
            FontLoadStatus::Cached => write!(f, "cached"),
        }
    }
}
