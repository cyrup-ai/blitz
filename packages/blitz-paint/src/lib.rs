//! Paint a [`blitz_dom::BaseDocument`] by pushing [`anyrender`] drawing commands into
//! an impl [`anyrender::PaintScene`].

mod color;
mod debug_overlay;
mod gradient;
mod layers;
mod multicolor_rounded_rect;
mod non_uniform_rounded_rect;
mod render;
pub mod screenshot;
mod sizing;
mod text;

use anyrender::PaintScene;
use blitz_dom::BaseDocument;
use layers::reset_layer_stats;
use render::BlitzDomPainter;
// Re-export screenshot types for public API
pub use screenshot::{
    ScreenshotConfig, ScreenshotConfigBuilder, ScreenshotEngine, ScreenshotRequest,
};

/// Paint a [`blitz_dom::BaseDocument`] by pushing drawing commands into
/// an impl [`anyrender::PaintScene`].
///
/// This function assumes that the styles and layout in the [`BaseDocument`] are already
/// resolved. Please ensure that this is the case before trying to paint.
///
/// The implementation of [`PaintScene`] is responsible for handling the commands that are pushed into it.
/// Generally this will involve executing them to draw a rasterized image/texture. But in some cases it may choose to
/// transform them to a vector format (e.g. SVG/PDF) or serialize them in raw form for later use.
pub fn paint_scene(
    scene: &mut impl PaintScene,
    dom: &BaseDocument,
    scale: f64,
    width: u32,
    height: u32,
) {
    reset_layer_stats();

    let devtools = *dom.devtools();
    let mut generator = BlitzDomPainter::new(dom, width, height, scale);
    generator.devtools = devtools;
    generator.paint_scene(scene);
}

/// Paint a [`blitz_dom::BaseDocument`] with screenshot capabilities
///
/// This function is similar to [`paint_scene`] but includes screenshot capture functionality.
/// The screenshot engine can process capture requests after rendering is complete.
///
/// # Arguments
///
/// * `scene` - The paint scene to render into
/// * `dom` - The document to render
/// * `scale` - The rendering scale factor
/// * `width` - The viewport width
/// * `height` - The viewport height
/// * `screenshot_engine` - Optional screenshot engine for capture functionality
///
/// # Returns
///
/// A painter instance that can be used for additional screenshot operations
pub fn paint_scene_with_screenshot<'dom>(
    scene: &mut impl PaintScene,
    dom: &'dom BaseDocument,
    scale: f64,
    width: u32,
    height: u32,
    screenshot_engine: Option<std::sync::Arc<ScreenshotEngine>>,
) -> BlitzDomPainter<'dom> {
    reset_layer_stats();

    let devtools = *dom.devtools();
    let mut generator = if let Some(engine) = screenshot_engine {
        BlitzDomPainter::new_with_screenshot_engine(dom, width, height, scale, engine)
    } else {
        BlitzDomPainter::new(dom, width, height, scale)
    };
    generator.devtools = devtools;
    generator.paint_scene(scene);
    generator
}
