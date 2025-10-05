//! Embedded fallback font to guarantee font_id validity
//!
//! This font is loaded into every FontSystem database to ensure
//! that font fallback ALWAYS succeeds, making font_ids effectively infallible.

use cosmyc_text::fontdb::{Database, ID, Source};
use std::sync::Arc;

/// Embedded fallback font data (moz-bullet-font.otf)
pub const EMBEDDED_FALLBACK_FONT: &[u8] = include_bytes!("../assets/fallback.otf");

/// Font family name for the embedded fallback
pub const EMBEDDED_FALLBACK_FAMILY: &str = "BlitzFallback";

/// Load the embedded fallback font into a fontdb database
///
/// Returns the font ID of the loaded fallback font.
///
/// # Implementation Note
/// Uses `load_font_source()` which returns a TinyVec of font IDs.
/// For single-face fonts like our embedded fallback, this returns one ID.
pub fn load_embedded_fallback(db: &mut Database) -> ID {
    let source = Source::Binary(Arc::new(EMBEDDED_FALLBACK_FONT.to_vec()));
    let font_ids = db.load_font_source(source);

    // Get the first (and only) font ID from the loaded font
    // SAFETY: We just loaded a valid font, so font_ids is non-empty
    font_ids
        .first()
        .copied()
        .expect("Embedded fallback font must load successfully")
}

/// Ensure a FontSystem has the embedded fallback font loaded
///
/// This is idempotent - safe to call multiple times.
/// Returns the font ID of the embedded fallback.
pub fn ensure_embedded_fallback(font_system: &mut cosmyc_text::FontSystem) -> ID {
    // Check if "BlitzFallback" font already exists in database
    let db = font_system.db();

    for face in db.faces() {
        if face.families.iter().any(|(name, _)| name == EMBEDDED_FALLBACK_FAMILY) {
            return face.id;
        }
    }

    // Not found - load it
    load_embedded_fallback(font_system.db_mut())
}
