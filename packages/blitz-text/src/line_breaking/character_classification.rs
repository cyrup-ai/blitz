//! Unicode character classification for line breaking
//!
//! This module handles the classification of Unicode characters into
//! UAX #14 line breaking classes with optimized ASCII fast paths.

use super::types::LineBreakClass;
use crate::error::ShapingError;

/// Classify a Unicode character for line break properties
pub fn classify_character_unicode(ch: char) -> LineBreakClass {
    match ch {
        // Control characters
        '\u{0000}'..='\u{0008}' => LineBreakClass::CM,
        '\u{0009}' => LineBreakClass::BA, // Tab
        '\u{000A}' => LineBreakClass::LF, // Line Feed
        '\u{000B}'..='\u{000C}' => LineBreakClass::BK,
        '\u{000D}' => LineBreakClass::CR, // Carriage Return
        '\u{000E}'..='\u{001F}' => LineBreakClass::CM,

        // Basic Latin punctuation and symbols
        ' ' => LineBreakClass::SP,
        '!' => LineBreakClass::EX,
        '"' => LineBreakClass::QU,
        '#' => LineBreakClass::AL,
        '$' => LineBreakClass::PR,
        '%' => LineBreakClass::PO,
        '&' => LineBreakClass::AL,
        '\'' => LineBreakClass::QU,
        '(' => LineBreakClass::OP,
        ')' => LineBreakClass::CL,
        '*' => LineBreakClass::AL,
        '+' => LineBreakClass::PR,
        ',' => LineBreakClass::IS,
        '-' => LineBreakClass::HY,
        '.' => LineBreakClass::IS,
        '/' => LineBreakClass::SY,
        '0'..='9' => LineBreakClass::NU,
        ':' => LineBreakClass::IS,
        ';' => LineBreakClass::IS,
        '<' => LineBreakClass::AL,
        '=' => LineBreakClass::AL,
        '>' => LineBreakClass::AL,
        '?' => LineBreakClass::EX,
        '@' => LineBreakClass::AL,
        'A'..='Z' => LineBreakClass::AL,
        '[' => LineBreakClass::OP,
        '\\' => LineBreakClass::PR,
        ']' => LineBreakClass::CL,
        '^' => LineBreakClass::AL,
        '_' => LineBreakClass::AL,
        '`' => LineBreakClass::AL,
        'a'..='z' => LineBreakClass::AL,
        '{' => LineBreakClass::OP,
        '|' => LineBreakClass::BA,
        '}' => LineBreakClass::CL,
        '~' => LineBreakClass::AL,
        '\u{007F}' => LineBreakClass::CM,

        // Unicode ranges (simplified classification)
        '\u{00A0}' => LineBreakClass::GL, // Non-breaking space
        '\u{00A1}' => LineBreakClass::OP, // Inverted exclamation
        '\u{00A2}'..='\u{00A5}' => LineBreakClass::PO, // Currency symbols
        '\u{00A6}' => LineBreakClass::AL, // Broken bar
        '\u{00A7}'..='\u{00A9}' => LineBreakClass::AL, // Section, copyright, etc.
        '\u{00AA}' => LineBreakClass::AL, // Feminine ordinal
        '\u{00AB}' => LineBreakClass::QU, // Left-pointing double angle quotation mark
        '\u{00AC}' => LineBreakClass::AL, // Not sign
        '\u{00AD}' => LineBreakClass::BA, // Soft hyphen
        '\u{00AE}'..='\u{00B1}' => LineBreakClass::AL,
        '\u{00B2}'..='\u{00B3}' => LineBreakClass::AL, // Superscript
        '\u{00B4}' => LineBreakClass::BB,              // Acute accent
        '\u{00B5}' => LineBreakClass::AL,              // Micro sign
        '\u{00B6}'..='\u{00B7}' => LineBreakClass::AL,
        '\u{00B8}' => LineBreakClass::AL,              // Cedilla
        '\u{00B9}' => LineBreakClass::AL,              // Superscript one
        '\u{00BA}' => LineBreakClass::AL,              // Masculine ordinal
        '\u{00BB}' => LineBreakClass::QU, // Right-pointing double angle quotation mark
        '\u{00BC}'..='\u{00BE}' => LineBreakClass::AL, // Fractions
        '\u{00BF}' => LineBreakClass::OP, // Inverted question mark

        // Extended Latin (simplified)
        '\u{00C0}'..='\u{00FF}' => LineBreakClass::AL,
        '\u{0100}'..='\u{017F}' => LineBreakClass::AL,

        // Greek and Coptic
        '\u{0370}'..='\u{03FF}' => LineBreakClass::AL,

        // Cyrillic
        '\u{0400}'..='\u{04FF}' => LineBreakClass::AL,

        // Hebrew
        '\u{0590}'..='\u{05FF}' => LineBreakClass::HL,

        // Arabic
        '\u{0600}'..='\u{06FF}' => LineBreakClass::AL,

        // CJK Unified Ideographs
        '\u{4E00}'..='\u{9FFF}' => LineBreakClass::ID,

        // Hangul
        '\u{AC00}'..='\u{D7AF}' => LineBreakClass::H2,

        // Emoji ranges (simplified)
        '\u{1F600}'..='\u{1F64F}' => LineBreakClass::EB, // Emoticons
        '\u{1F680}'..='\u{1F6FF}' => LineBreakClass::EB, // Transport and map symbols
        '\u{1F700}'..='\u{1F77F}' => LineBreakClass::EB, // Alchemical symbols
        '\u{1F780}'..='\u{1F7FF}' => LineBreakClass::EB, // Geometric shapes extended
        '\u{1F800}'..='\u{1F8FF}' => LineBreakClass::EB, // Supplemental arrows C
        '\u{1F900}'..='\u{1F9FF}' => LineBreakClass::EB, // Supplemental symbols and pictographs

        // Zero-width characters
        '\u{200B}' => LineBreakClass::ZW,  // Zero-width space
        '\u{200C}' => LineBreakClass::CM,  // Zero-width non-joiner
        '\u{200D}' => LineBreakClass::ZWJ, // Zero-width joiner
        '\u{200E}'..='\u{200F}' => LineBreakClass::CM, // Directional marks
        '\u{2060}' => LineBreakClass::WJ,  // Word joiner
        '\u{FEFF}' => LineBreakClass::WJ,  // Zero-width no-break space

        // Default classification for unhandled characters
        _ => LineBreakClass::AL,
    }
}

/// Extension trait for character classification
pub trait CharacterExtensions {
    fn is_ideographic(&self) -> bool;
}

impl CharacterExtensions for char {
    fn is_ideographic(&self) -> bool {
        // CJK Unified Ideographs and related blocks
        matches!(*self,
            '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
            '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
            '\u{20000}'..='\u{2A6DF}' | // CJK Extension B
            '\u{2A700}'..='\u{2B73F}' | // CJK Extension C
            '\u{2B740}'..='\u{2B81F}' | // CJK Extension D
            '\u{2B820}'..='\u{2CEAF}' | // CJK Extension E
            '\u{2CEB0}'..='\u{2EBEF}' | // CJK Extension F
            '\u{30000}'..='\u{3134F}'   // CJK Extension G
        )
    }
}

/// Populate ASCII character properties for fast lookup
pub fn populate_ascii_properties(cache: &mut [LineBreakClass; 256]) {
    for (i, class) in cache.iter_mut().enumerate() {
        let ch = i as u8 as char;
        *class = match ch {
            '\t' => LineBreakClass::BA,
            '\n' => LineBreakClass::LF,
            '\r' => LineBreakClass::CR,
            ' ' => LineBreakClass::SP,
            '!' => LineBreakClass::EX,
            '"' => LineBreakClass::QU,
            '%' => LineBreakClass::PO,
            '\'' => LineBreakClass::QU,
            '(' => LineBreakClass::OP,
            ')' => LineBreakClass::CL,
            ',' => LineBreakClass::IS,
            '-' => LineBreakClass::HY,
            '.' => LineBreakClass::IS,
            '/' => LineBreakClass::SY,
            '0'..='9' => LineBreakClass::NU,
            ':' | ';' => LineBreakClass::IS,
            '?' => LineBreakClass::EX,
            'A'..='Z' | 'a'..='z' => LineBreakClass::AL,
            '[' => LineBreakClass::OP,
            ']' => LineBreakClass::CL,
            '{' => LineBreakClass::OP,
            '|' => LineBreakClass::BA,
            '}' => LineBreakClass::CL,
            _ => LineBreakClass::AL,
        };
    }
}

/// SIMD-accelerated character classification for line break properties
pub fn classify_characters_simd(
    chars: &[char],
    break_property_cache: &[LineBreakClass; 256],
) -> Result<Vec<LineBreakClass>, ShapingError> {
    let mut classes = Vec::with_capacity(chars.len());

    // Process characters in SIMD-friendly chunks
    const CHUNK_SIZE: usize = 64;

    for chunk in chars.chunks(CHUNK_SIZE) {
        // Fast path for ASCII characters
        if chunk.iter().all(|&c| c.is_ascii()) {
            for &ch in chunk {
                classes.push(break_property_cache[ch as usize]);
            }
        } else {
            // Full Unicode classification for non-ASCII
            for &ch in chunk {
                classes.push(classify_character_unicode(ch));
            }
        }
    }

    Ok(classes)
}
