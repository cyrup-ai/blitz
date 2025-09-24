use std::ops::RangeInclusive;
use std::path::Path;

use glyphon::cosmyc_text::{Stretch, Style, Weight};

use crate::{FontError, SystemFont};

/// Result of font parsing operations
#[derive(Debug, Clone)]
pub struct FontParseResult {
    pub font: Option<SystemFont>,
    pub parse_time_ms: u64,
    pub file_size_bytes: u64,
}

/// Font file parser for extracting metadata
pub struct FontParser;

impl FontParser {
    /// Parse font file and extract SystemFont metadata
    pub fn parse_font_file(path: &Path) -> Result<Option<SystemFont>, FontError> {
        // Check file size first to avoid loading huge files
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        if file_size < crate::constants::MIN_FONT_FILE_SIZE
            || file_size > crate::constants::MAX_FONT_FILE_SIZE
        {
            return Ok(None);
        }

        let data = std::fs::read(path)?;
        Self::parse_font_data(&data, path)
    }

    /// Parse font data and extract SystemFont metadata
    pub fn parse_font_data(data: &[u8], path: &Path) -> Result<Option<SystemFont>, FontError> {
        use ttf_parser::Face;

        let face = Face::parse(data, 0)?;

        let family = face
            .names()
            .into_iter()
            .find(|name| name.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|name| name.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let weight = Weight(face.weight().to_number());
        let style = if face.is_italic() {
            Style::Italic
        } else {
            Style::Normal
        };
        let stretch = Self::convert_width_to_stretch(face.width());

        let is_monospace = face.is_monospaced();
        let supports_emoji = Self::check_emoji_support(&face);
        let unicode_ranges = Self::extract_unicode_ranges(&face);

        Ok(Some(SystemFont {
            family,
            path: path.to_path_buf(),
            weight,
            style,
            stretch,
            is_monospace,
            supports_emoji,
            unicode_ranges,
        }))
    }

    /// Convert ttf_parser Width to cosmyc_text Stretch
    fn convert_width_to_stretch(width: ttf_parser::Width) -> Stretch {
        match width {
            ttf_parser::Width::UltraCondensed => Stretch::UltraCondensed,
            ttf_parser::Width::ExtraCondensed => Stretch::ExtraCondensed,
            ttf_parser::Width::Condensed => Stretch::Condensed,
            ttf_parser::Width::SemiCondensed => Stretch::SemiCondensed,
            ttf_parser::Width::Normal => Stretch::Normal,
            ttf_parser::Width::SemiExpanded => Stretch::SemiExpanded,
            ttf_parser::Width::Expanded => Stretch::Expanded,
            ttf_parser::Width::ExtraExpanded => Stretch::ExtraExpanded,
            ttf_parser::Width::UltraExpanded => Stretch::UltraExpanded,
        }
    }

    /// Check emoji support by examining font tables
    #[inline]
    fn check_emoji_support(face: &ttf_parser::Face) -> bool {
        // Check for color emoji tables (COLR) or bitmap emoji tables (CBDT)
        face.tables().colr.is_some() || face.tables().cbdt.is_some()
    }

    /// Extract Unicode ranges with optimized performance
    fn extract_unicode_ranges(face: &ttf_parser::Face) -> Vec<RangeInclusive<u32>> {
        let mut ranges = Vec::with_capacity(8); // Pre-allocate for common case

        // Provide standard Unicode ranges based on font type detection
        // This is a simplified approach since OS/2 table access is complex

        // Check if font has various language support by checking character mappings
        if face.glyph_index('\u{0100}').is_some() {
            ranges.push(0x0100..=0x017F); // Latin Extended-A
        }

        if face.glyph_index('\u{0400}').is_some() {
            ranges.push(0x0400..=0x04FF); // Cyrillic
        }

        if face.glyph_index('\u{0370}').is_some() {
            ranges.push(0x0370..=0x03FF); // Greek and Coptic
        }

        if face.glyph_index('\u{0590}').is_some() {
            ranges.push(0x0590..=0x05FF); // Hebrew
        }

        if face.glyph_index('\u{0600}').is_some() {
            ranges.push(0x0600..=0x06FF); // Arabic
        }

        // Always include Basic Latin as fallback
        ranges.insert(0, 0x0000..=0x007F); // Basic Latin

        ranges
    }

    /// Parse with timing information for performance monitoring
    pub fn parse_with_timing(path: &Path) -> Result<FontParseResult, FontError> {
        let start = std::time::Instant::now();
        let metadata = std::fs::metadata(path)?;
        let file_size_bytes = metadata.len();

        let font = Self::parse_font_file(path)?;
        let parse_time_ms = start.elapsed().as_millis() as u64;

        Ok(FontParseResult {
            font,
            parse_time_ms,
            file_size_bytes,
        })
    }

    /// Check if a font supports a specific Unicode codepoint
    pub fn font_supports_codepoint(path: &Path, codepoint: u32) -> Result<bool, FontError> {
        let data = std::fs::read(path)?;
        Self::font_data_supports_codepoint(&data, codepoint)
    }

    /// Check if font data supports a specific Unicode codepoint
    pub fn font_data_supports_codepoint(data: &[u8], codepoint: u32) -> Result<bool, FontError> {
        use ttf_parser::Face;

        let face = Face::parse(data, 0)?;
        let char = char::from_u32(codepoint)
            .ok_or_else(|| FontError::InvalidFormat("Invalid Unicode codepoint".to_string()))?;
        Ok(face.glyph_index(char).is_some())
    }

    /// Extract basic font metrics for layout calculations
    pub fn extract_basic_metrics(path: &Path) -> Result<Option<crate::FontMetrics>, FontError> {
        let data = std::fs::read(path)?;
        Self::extract_metrics_from_data(&data)
    }

    /// Extract font metrics from font data
    pub fn extract_metrics_from_data(data: &[u8]) -> Result<Option<crate::FontMetrics>, FontError> {
        use ttf_parser::Face;

        let face = Face::parse(data, 0)?;

        // ✅ USE EXISTING OPTIMIZED IMPLEMENTATION instead of manual construction
        Ok(Some(crate::FontMetrics::from_face(&face)))
    }
}
