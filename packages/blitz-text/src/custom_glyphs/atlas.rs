//! Atlas processing and coordinate management
//!
//! This module handles texture atlas operations, coordinate calculations,
//! and embedded atlas data for emoji and icon rendering.

use std::sync::OnceLock;

use image::{imageops, ImageFormat};

use super::types::{AtlasCoords, AtlasMetadata, CustomGlyphError};

/// Embedded atlas data for emoji rendering
const EMOJI_ATLAS_DATA: &[u8] = include_bytes!("../../assets/emoji_atlas.png");

/// Embedded atlas data for private use icons
const ICON_ATLAS_DATA: &[u8] = include_bytes!("../../assets/icon_atlas.png");

/// Cached decoded emoji atlas image (decoded once at first access)
static CACHED_EMOJI_ATLAS: OnceLock<AtlasMetadata> = OnceLock::new();

/// Cached decoded icon atlas image (decoded once at first access)
static CACHED_ICON_ATLAS: OnceLock<AtlasMetadata> = OnceLock::new();

/// Emoji atlas metadata - covers emoticons range 0x1F600-0x1F64F (80 glyphs)
const EMOJI_ATLAS_METADATA: AtlasMetadata = AtlasMetadata {
    width: 512,
    height: 320,
    glyph_width: 32,
    glyph_height: 32,
    glyphs_per_row: 16,
    rows: 10,
};

/// Icon atlas metadata - covers private use range 0xE000-0xE0FF (256 glyphs)
const ICON_ATLAS_METADATA: AtlasMetadata = AtlasMetadata {
    width: 512,
    height: 512,
    glyph_width: 32,
    glyph_height: 32,
    glyphs_per_row: 16,
    rows: 16,
};

/// Calculate atlas coordinates from Unicode codepoint for emoji range
#[inline(always)]
fn calculate_emoji_atlas_coords(codepoint: u32) -> Option<(u32, u32, u32, u32)> {
    if !(0x1F600..=0x1F64F).contains(&codepoint) {
        return None;
    }

    let index = codepoint - 0x1F600;
    let row = index / EMOJI_ATLAS_METADATA.glyphs_per_row;
    let col = index % EMOJI_ATLAS_METADATA.glyphs_per_row;

    if row >= EMOJI_ATLAS_METADATA.rows {
        return None;
    }

    let x = col * EMOJI_ATLAS_METADATA.glyph_width;
    let y = row * EMOJI_ATLAS_METADATA.glyph_height;

    Some((
        x,
        y,
        EMOJI_ATLAS_METADATA.glyph_width,
        EMOJI_ATLAS_METADATA.glyph_height,
    ))
}

/// Calculate atlas coordinates from Unicode codepoint for private use range
#[inline(always)]
fn calculate_icon_atlas_coords(codepoint: u32) -> Option<(u32, u32, u32, u32)> {
    if !(0xE000..=0xE0FF).contains(&codepoint) {
        return None;
    }

    let index = codepoint - 0xE000;
    let row = index / ICON_ATLAS_METADATA.glyphs_per_row;
    let col = index % ICON_ATLAS_METADATA.glyphs_per_row;

    if row >= ICON_ATLAS_METADATA.rows {
        return None;
    }

    let x = col * ICON_ATLAS_METADATA.glyph_width;
    let y = row * ICON_ATLAS_METADATA.glyph_height;

    Some((
        x,
        y,
        ICON_ATLAS_METADATA.glyph_width,
        ICON_ATLAS_METADATA.glyph_height,
    ))
}

/// Get cached emoji atlas metadata
#[inline]
fn get_cached_emoji_atlas() -> &'static AtlasMetadata {
    CACHED_EMOJI_ATLAS.get_or_init(|| {
        // In a real implementation, this would decode the PNG and extract metadata
        // For now, we return the static metadata
        EMOJI_ATLAS_METADATA
    })
}

/// Get cached icon atlas metadata
#[inline]
fn get_cached_icon_atlas() -> &'static AtlasMetadata {
    CACHED_ICON_ATLAS.get_or_init(|| {
        // In a real implementation, this would decode the PNG and extract metadata
        // For now, we return the static metadata
        ICON_ATLAS_METADATA
    })
}

/// Extract specific glyph from atlas data
#[inline]
fn extract_emoji_from_atlas(
    atlas_data: &[u8],
    codepoint: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, CustomGlyphError> {
    // 1. Calculate coordinates using existing function
    let (x, y, w, h) = calculate_emoji_atlas_coords(codepoint)
        .ok_or(CustomGlyphError::CoordinateCalculationFailed(codepoint))?;

    // 2. Load PNG from memory
    let img = image::load_from_memory_with_format(atlas_data, ImageFormat::Png)
        .map_err(|e| CustomGlyphError::AtlasDecodeError(format!("PNG decode failed: {}", e)))?;

    // 3. Extract glyph region
    let cropped = imageops::crop_imm(&img, x, y, w, h);

    // 4. Convert to RGBA and scale if needed
    let mut rgba_img = cropped.to_image();

    // 5. Resize to requested dimensions if different from atlas glyph size
    if width != w || height != h {
        rgba_img = image::imageops::resize(
            &rgba_img,
            width,
            height,
            image::imageops::FilterType::Lanczos3,
        );
    }

    // 6. Return raw RGBA bytes
    Ok(rgba_img.into_raw())
}

/// Extract specific icon from atlas data
#[inline]
fn extract_icon_from_atlas(
    atlas_data: &[u8],
    codepoint: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, CustomGlyphError> {
    // 1. Calculate coordinates using existing function
    let (x, y, w, h) = calculate_icon_atlas_coords(codepoint)
        .ok_or(CustomGlyphError::CoordinateCalculationFailed(codepoint))?;

    // 2. Load PNG from memory
    let img = image::load_from_memory_with_format(atlas_data, ImageFormat::Png)
        .map_err(|e| CustomGlyphError::AtlasDecodeError(format!("PNG decode failed: {}", e)))?;

    // 3. Extract icon region
    let cropped = imageops::crop_imm(&img, x, y, w, h);

    // 4. Convert to RGBA and scale if needed
    let mut rgba_img = cropped.to_image();

    // 5. Resize to requested dimensions if different from atlas glyph size
    if width != w || height != h {
        rgba_img = image::imageops::resize(
            &rgba_img,
            width,
            height,
            image::imageops::FilterType::Lanczos3,
        );
    }

    // 6. Return raw RGBA bytes
    Ok(rgba_img.into_raw())
}

/// Extract emoji glyph data from embedded atlas
#[inline]
fn extract_emoji_glyph(
    codepoint: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, CustomGlyphError> {
    extract_emoji_from_atlas(EMOJI_ATLAS_DATA, codepoint, width, height)
}

/// Extract icon glyph data from embedded atlas
#[inline]
fn extract_icon_glyph(
    codepoint: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, CustomGlyphError> {
    extract_icon_from_atlas(ICON_ATLAS_DATA, codepoint, width, height)
}

/// Atlas processing utilities
pub struct AtlasProcessor;

impl AtlasProcessor {
    /// Get emoji atlas coordinates for a Unicode codepoint
    pub fn get_emoji_coords(codepoint: u32) -> Option<AtlasCoords> {
        let (x, y, w, h) = calculate_emoji_atlas_coords(codepoint)?;
        Some(AtlasCoords {
            x: x as u16,
            y: y as u16,
            width: w as u16,
            height: h as u16,
        })
    }

    /// Get icon atlas coordinates for a Unicode codepoint
    pub fn get_icon_coords(codepoint: u32) -> Option<AtlasCoords> {
        let (x, y, w, h) = calculate_icon_atlas_coords(codepoint)?;
        Some(AtlasCoords {
            x: x as u16,
            y: y as u16,
            width: w as u16,
            height: h as u16,
        })
    }

    /// Extract emoji glyph data
    pub fn extract_emoji(
        codepoint: u32,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, CustomGlyphError> {
        extract_emoji_glyph(codepoint, width, height)
    }

    /// Extract icon glyph data
    pub fn extract_icon(
        codepoint: u32,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, CustomGlyphError> {
        extract_icon_glyph(codepoint, width, height)
    }

    /// Get emoji atlas metadata
    pub fn emoji_atlas_metadata() -> &'static AtlasMetadata {
        get_cached_emoji_atlas()
    }

    /// Get icon atlas metadata
    pub fn icon_atlas_metadata() -> &'static AtlasMetadata {
        get_cached_icon_atlas()
    }

    /// Check if codepoint is in emoji range
    pub fn is_emoji_codepoint(codepoint: u32) -> bool {
        (0x1F600..=0x1F64F).contains(&codepoint)
    }

    /// Check if codepoint is in icon range
    pub fn is_icon_codepoint(codepoint: u32) -> bool {
        (0xE000..=0xE0FF).contains(&codepoint)
    }

    /// Get atlas coordinates for any supported codepoint
    pub fn get_coords_for_codepoint(codepoint: u32) -> Option<AtlasCoords> {
        if Self::is_emoji_codepoint(codepoint) {
            Self::get_emoji_coords(codepoint)
        } else if Self::is_icon_codepoint(codepoint) {
            Self::get_icon_coords(codepoint)
        } else {
            None
        }
    }

    /// Extract glyph data for any supported codepoint
    pub fn extract_glyph_for_codepoint(
        codepoint: u32,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, CustomGlyphError> {
        if Self::is_emoji_codepoint(codepoint) {
            Self::extract_emoji(codepoint, width, height)
        } else if Self::is_icon_codepoint(codepoint) {
            Self::extract_icon(codepoint, width, height)
        } else {
            Err(CustomGlyphError::CoordinateCalculationFailed(codepoint))
        }
    }

    /// Validate atlas coordinates
    pub fn validate_coords(coords: &AtlasCoords, atlas_metadata: &AtlasMetadata) -> bool {
        coords.x as u32 + coords.width as u32 <= atlas_metadata.width
            && coords.y as u32 + coords.height as u32 <= atlas_metadata.height
    }

    /// Calculate atlas utilization
    pub fn calculate_utilization(used_glyphs: u32, atlas_metadata: &AtlasMetadata) -> f32 {
        let total_glyphs = atlas_metadata.total_glyphs();
        if total_glyphs > 0 {
            used_glyphs as f32 / total_glyphs as f32
        } else {
            0.0
        }
    }
}

/// Emoji range U+1F600-1F64F maps to 16x16 grid positions
fn emoji_codepoint_to_grid_pos(codepoint: u32) -> Option<(u32, u32)> {
    match codepoint {
        0x1F600..=0x1F64F => {
            let index = codepoint - 0x1F600;
            let row = index / 16;
            let col = index % 16;
            Some((col, row))
        }
        _ => None,
    }
}

/// Private use area U+E000-F8FF maps to icon grid positions
fn icon_codepoint_to_grid_pos(codepoint: u32) -> Option<(u32, u32)> {
    match codepoint {
        0xE000..=0xE0FF => {
            let index = codepoint - 0xE000;
            let row = index / 16;
            let col = index % 16;
            Some((col, row))
        }
        _ => None,
    }
}

/// Get emoji atlas dimensions for rendering setup
pub fn get_emoji_atlas_dimensions() -> (u32, u32) {
    let atlas = get_cached_emoji_atlas();
    (atlas.width, atlas.height)
}

/// Get icon atlas dimensions for rendering setup
pub fn get_icon_atlas_dimensions() -> (u32, u32) {
    let atlas = get_cached_icon_atlas();
    (atlas.width, atlas.height)
}

/// Calculate texture coordinates for a specific emoji
pub fn calculate_emoji_texture_coords(emoji_char: char) -> Option<(f32, f32)> {
    calculate_emoji_atlas_coords(emoji_char as u32).map(|(x, y, _, _)| (x as f32, y as f32))
}

/// Calculate texture coordinates for a specific icon
pub fn calculate_icon_texture_coords(icon_type: char) -> Option<(f32, f32)> {
    calculate_icon_atlas_coords(icon_type as u32).map(|(x, y, _, _)| (x as f32, y as f32))
}
