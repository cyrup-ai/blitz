use std::path::PathBuf;

use glyphon::cosmyc_text::{Stretch, Style, Weight};

use crate::FontKey;

/// System font information discovered during font scanning
#[derive(Debug, Clone)]
pub struct SystemFont {
    pub family: String,
    pub path: PathBuf,
    pub weight: Weight,
    pub style: Style,
    pub stretch: Stretch,
    pub is_monospace: bool,
    pub supports_emoji: bool,
    pub unicode_ranges: Vec<std::ops::RangeInclusive<u32>>,
}

impl SystemFont {
    /// Create a new SystemFont instance
    pub fn new(
        family: String,
        path: PathBuf,
        weight: Weight,
        style: Style,
        stretch: Stretch,
    ) -> Self {
        Self {
            family,
            path,
            weight,
            style,
            stretch,
            is_monospace: false,
            supports_emoji: false,
            unicode_ranges: Vec::new(),
        }
    }

    /// Create a FontKey for this system font
    pub fn to_font_key(&self) -> FontKey {
        FontKey {
            family: self.family.clone(),
            weight: self.weight,
            style: self.style,
            stretch: self.stretch,
        }
    }

    /// Check if this font supports a specific Unicode codepoint
    pub fn supports_codepoint(&self, codepoint: u32) -> bool {
        self.unicode_ranges
            .iter()
            .any(|range| range.contains(&codepoint))
    }

    /// Get the file size of this font
    pub fn file_size(&self) -> Result<u64, std::io::Error> {
        let metadata = std::fs::metadata(&self.path)?;
        Ok(metadata.len())
    }

    /// Get the file extension
    pub fn file_extension(&self) -> Option<String> {
        self.path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// Check if this is a variable font
    pub fn is_variable_font(&self) -> Result<bool, crate::FontError> {
        let data = std::fs::read(&self.path)?;
        self.check_variable_font_from_data(&data)
    }

    fn check_variable_font_from_data(&self, data: &[u8]) -> Result<bool, crate::FontError> {
        use ttf_parser::Face;

        let face = Face::parse(data, 0)?;
        Ok(face.tables().fvar.is_some())
    }

    /// Get font capabilities summary
    pub fn capabilities(&self) -> FontCapabilities {
        FontCapabilities {
            monospace: self.is_monospace,
            emoji: self.supports_emoji,
            variable_font: self.is_variable_font().unwrap_or(false),
            unicode_range_count: self.unicode_ranges.len(),
            supported_scripts: self.get_supported_scripts(),
        }
    }

    /// Get list of supported writing scripts
    pub fn get_supported_scripts(&self) -> Vec<WritingScript> {
        let mut scripts = Vec::new();

        for range in &self.unicode_ranges {
            if let Some(script) = WritingScript::from_unicode_range(range) {
                if !scripts.contains(&script) {
                    scripts.push(script);
                }
            }
        }

        scripts.sort();
        scripts
    }

    /// Check if font supports a specific writing script
    pub fn supports_script(&self, script: WritingScript) -> bool {
        self.get_supported_scripts().contains(&script)
    }

    /// Get a quality score for this font (0.0 = poor, 1.0 = excellent)
    pub fn quality_score(&self) -> f32 {
        let mut score = 0.0;

        // Base score for existing
        score += 0.3;

        // Bonus for emoji support
        if self.supports_emoji {
            score += 0.1;
        }

        // Bonus for wide unicode coverage
        let range_count = self.unicode_ranges.len() as f32;
        score += (range_count / 64.0).min(0.3); // Max 0.3 for good coverage

        // Bonus for monospace fonts (useful for code)
        if self.is_monospace {
            score += 0.1;
        }

        // Bonus for variable fonts
        if self.is_variable_font().unwrap_or(false) {
            score += 0.1;
        }

        // Bonus for common file formats
        match self.file_extension().as_deref() {
            Some("ttf") | Some("otf") => score += 0.1,
            Some("woff2") => score += 0.05,
            _ => {}
        }

        score.min(1.0)
    }
}

/// Font capabilities summary
#[derive(Debug, Clone)]
pub struct FontCapabilities {
    pub monospace: bool,
    pub emoji: bool,
    pub variable_font: bool,
    pub unicode_range_count: usize,
    pub supported_scripts: Vec<WritingScript>,
}

/// Writing scripts supported by fonts
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WritingScript {
    Latin,
    Cyrillic,
    Greek,
    Arabic,
    Hebrew,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Thai,
    Lao,
    Georgian,
    Hangul,
    Hiragana,
    Katakana,
    CJKIdeographs,
    Unknown,
}

impl WritingScript {
    /// Determine script from unicode range
    pub fn from_unicode_range(range: &std::ops::RangeInclusive<u32>) -> Option<Self> {
        let start = *range.start();
        let end = *range.end();

        if start <= 0x007F {
            Some(WritingScript::Latin)
        } else if start <= 0x04FF && end >= 0x0400 {
            Some(WritingScript::Cyrillic)
        } else if start <= 0x03FF && end >= 0x0370 {
            Some(WritingScript::Greek)
        } else if start <= 0x06FF && end >= 0x0600 {
            Some(WritingScript::Arabic)
        } else if start <= 0x05FF && end >= 0x0590 {
            Some(WritingScript::Hebrew)
        } else if start <= 0x097F && end >= 0x0900 {
            Some(WritingScript::Devanagari)
        } else if start <= 0x09FF && end >= 0x0980 {
            Some(WritingScript::Bengali)
        } else if start <= 0x0A7F && end >= 0x0A00 {
            Some(WritingScript::Gurmukhi)
        } else if start <= 0x0AFF && end >= 0x0A80 {
            Some(WritingScript::Gujarati)
        } else if start <= 0x0B7F && end >= 0x0B00 {
            Some(WritingScript::Oriya)
        } else if start <= 0x0BFF && end >= 0x0B80 {
            Some(WritingScript::Tamil)
        } else if start <= 0x0C7F && end >= 0x0C00 {
            Some(WritingScript::Telugu)
        } else if start <= 0x0CFF && end >= 0x0C80 {
            Some(WritingScript::Kannada)
        } else if start <= 0x0D7F && end >= 0x0D00 {
            Some(WritingScript::Malayalam)
        } else if start <= 0x0E7F && end >= 0x0E00 {
            Some(WritingScript::Thai)
        } else if start <= 0x0EFF && end >= 0x0E80 {
            Some(WritingScript::Lao)
        } else if start <= 0x10FF && end >= 0x10A0 {
            Some(WritingScript::Georgian)
        } else if start <= 0xD7AF && end >= 0xAC00 {
            Some(WritingScript::Hangul)
        } else if start <= 0x309F && end >= 0x3040 {
            Some(WritingScript::Hiragana)
        } else if start <= 0x30FF && end >= 0x30A0 {
            Some(WritingScript::Katakana)
        } else if start <= 0x9FFF && end >= 0x4E00 {
            Some(WritingScript::CJKIdeographs)
        } else {
            None
        }
    }

    /// Get human-readable name for the script
    pub fn display_name(&self) -> &'static str {
        match self {
            WritingScript::Latin => "Latin",
            WritingScript::Cyrillic => "Cyrillic",
            WritingScript::Greek => "Greek",
            WritingScript::Arabic => "Arabic",
            WritingScript::Hebrew => "Hebrew",
            WritingScript::Devanagari => "Devanagari",
            WritingScript::Bengali => "Bengali",
            WritingScript::Gurmukhi => "Gurmukhi",
            WritingScript::Gujarati => "Gujarati",
            WritingScript::Oriya => "Oriya",
            WritingScript::Tamil => "Tamil",
            WritingScript::Telugu => "Telugu",
            WritingScript::Kannada => "Kannada",
            WritingScript::Malayalam => "Malayalam",
            WritingScript::Thai => "Thai",
            WritingScript::Lao => "Lao",
            WritingScript::Georgian => "Georgian",
            WritingScript::Hangul => "Hangul",
            WritingScript::Hiragana => "Hiragana",
            WritingScript::Katakana => "Katakana",
            WritingScript::CJKIdeographs => "CJK Ideographs",
            WritingScript::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for WritingScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
