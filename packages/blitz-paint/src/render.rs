mod background;
mod box_shadow;
mod form_controls;

use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};

use anyrender::{CustomPaint, Paint, PaintScene};
use blitz_dom::node::{
    ListItemLayout, ListItemLayoutPosition, Marker, NodeData, RasterImageData, TextInputData,
    TextNodeData,
};
use blitz_dom::{BaseDocument, ElementData, Node, local_name};
use blitz_text;
use blitz_traits::devtools::DevtoolSettings;
use euclid::Transform3D;
use kurbo::{self, Affine, Point, Rect, Stroke, Vec2};
use peniko::{self, Fill};
use style::color::AbsoluteColor;
use style::values::generics::color::GenericColor;
use style::values::generics::image::GenericImage;
use style::{
    dom::TElement,
    properties::{
        ComputedValues, generated::longhands::visibility::computed_value::T as StyloVisibility,
        style_structs::Font,
    },
    values::{
        computed::{CSSPixelLength, Overflow},
        specified::{BorderStyle, OutlineStyle, image::ImageRendering},
    },
};
use taffy::Layout;
use unicode_segmentation::UnicodeSegmentation;

use super::multicolor_rounded_rect::{Edge, ElementFrame};
use crate::color::{Color, ToColorColor};
use crate::debug_overlay::render_debug_overlay;
use crate::layers::maybe_with_layer;
use crate::screenshot::ScreenshotEngine;
use crate::sizing::compute_object_fit;

/// Alpha transparency threshold for visibility determination
/// Uses epsilon comparison for floating point precision
const ALPHA_VISIBILITY_THRESHOLD: f32 = f32::EPSILON;

/// Maximum allowed border width in pixels to prevent overflow
/// Based on CSS specification limits and practical rendering constraints
const MAX_BORDER_WIDTH_PX: f32 = 1000.0;

/// Default border width fallback for error cases
/// Provides graceful degradation when border width conversion fails
const DEFAULT_BORDER_WIDTH_PX: f32 = 1.0;

/// Composite key for cycle detection in render tree traversal
/// Includes node ID and quantized location to handle legitimate multiple rendering
/// while preventing infinite recursion loops
type RenderKey = (usize, i32, i32);

// Thread-local storage for render cycle detection to achieve zero allocation.
// Reuses the same HashSet across render passes after initial capacity growth.
thread_local! {
    static RENDER_VISITED: RefCell<HashSet<RenderKey>> = RefCell::new(HashSet::new());
}

/// Creates a render key from node ID and location
/// Uses rounded coordinates to prevent infinite recursion while allowing legitimate re-renders
#[inline(always)]
fn make_render_key(node_id: usize, location: Point) -> RenderKey {
    (
        node_id,
        location.x.round() as i32,
        location.y.round() as i32,
    )
}

/// A short-lived struct which holds a bunch of parameters for rendering a scene so
/// that we don't have to pass them down as parameters
/// Tracks the state of the current render pass
#[derive(Debug, Default)]
struct RenderState {
    /// Tracks nodes that have been rendered in the current pass
    rendered_nodes: HashSet<usize>,
    /// Current render pass number (incremented on each full render)
    pass: u64,
}

pub struct BlitzDomPainter<'dom> {
    /// Input parameters (read only) for generating the Scene
    pub(crate) dom: &'dom BaseDocument,
    pub(crate) scale: f64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) devtools: DevtoolSettings,
    /// Tracks render state across the current render pass
    render_state: Rc<RefCell<RenderState>>,
    /// Screenshot engine for capture functionality
    screenshot_engine: Option<Arc<ScreenshotEngine>>,
}

impl<'dom> BlitzDomPainter<'dom> {
    fn node_position(&self, node: usize, location: Point) -> (Layout, Point) {
        let layout = self.layout(node);
        let pos = location + Vec2::new(layout.location.x as f64, layout.location.y as f64);
        (layout, pos)
    }

    fn layout(&self, child: usize) -> Layout {
        self.dom.as_ref().tree()[child].unrounded_layout
        // self.dom.tree()[child].final_layout
    }

    /// Creates a new BlitzDomPainter with screenshot engine
    pub fn new(dom: &'dom BaseDocument, width: u32, height: u32, scale: f64) -> Self {
        Self {
            dom,
            width,
            height,
            scale,
            devtools: Default::default(),
            render_state: Rc::new(RefCell::new(RenderState::default())),
            screenshot_engine: None,
        }
    }

    pub fn new_with_screenshot_engine(
        dom: &'dom BaseDocument,
        width: u32,
        height: u32,
        scale: f64,
        screenshot_engine: Arc<ScreenshotEngine>,
    ) -> Self {
        Self {
            dom,
            width,
            height,
            scale,
            devtools: Default::default(),
            render_state: Rc::new(RefCell::new(RenderState::default())),
            screenshot_engine: Some(screenshot_engine),
        }
    }

    /// Set the screenshot engine for this painter
    #[inline]
    pub fn set_screenshot_engine(&mut self, engine: Arc<ScreenshotEngine>) {
        self.screenshot_engine = Some(engine);
    }

    /// Get reference to screenshot engine
    #[inline]
    pub fn screenshot_engine(&self) -> Option<&Arc<ScreenshotEngine>> {
        self.screenshot_engine.as_ref()
    }

    /// Ensures all styles are computed before rendering
    fn ensure_styles_computed(&self) {
        // Force style computation for the entire tree
        // This prevents style recalculations during rendering
        let _ = self.dom.as_ref().root_element().primary_styles();
    }

    /// Draw the current tree to current render surface
    /// Eventually we'll want the surface itself to be passed into the render function, along with things like the viewport
    ///
    /// This assumes styles are resolved and layout is complete.
    /// Make sure you do those before trying to render
    pub fn paint_scene(&self, scene: &mut impl PaintScene) {
        let _root_id = self.dom.as_ref().root_element().id;
        // Ensure all styles are computed before starting the render
        self.ensure_styles_computed();

        // Reset render state for new frame
        {
            let mut state = self.render_state.borrow_mut();
            state.rendered_nodes.clear();
            state.pass = state.pass.wrapping_add(1);
        }
        // Reset the scene and get viewport information
        scene.reset();
        let viewport_scroll = self.dom.as_ref().viewport_scroll();

        let root_element = self.dom.as_ref().root_element();
        let root_id = root_element.id;
        let bg_width = (self.width as f32).max(root_element.final_layout.size.width);
        let bg_height = (self.height as f32).max(root_element.final_layout.size.height);

        let background_color = {
            let html_color = root_element
                .primary_styles()
                .map(|s| s.clone_background_color())
                .unwrap_or(GenericColor::TRANSPARENT_BLACK);
            if html_color == GenericColor::TRANSPARENT_BLACK {
                root_element
                    .children
                    .iter()
                    .find_map(|id| {
                        self.dom
                            .as_ref()
                            .get_node(*id)
                            .filter(|node| node.data.is_element_with_tag_name(&local_name!("body")))
                    })
                    .and_then(|body| body.primary_styles())
                    .map(|style| {
                        let current_color = style.clone_color();
                        style
                            .clone_background_color()
                            .resolve_to_absolute(&current_color)
                    })
            } else {
                // Graceful handling: provide fallback if primary styles are missing
                let current_color = root_element
                    .primary_styles()
                    .map(|styles| styles.clone_color())
                    .unwrap_or_else(|| AbsoluteColor::BLACK); // CSS specification default
                Some(html_color.resolve_to_absolute(&current_color))
            }
        };

        if let Some(bg_color) = background_color {
            let bg_color = bg_color.as_srgb_color();
            let rect = Rect::from_origin_size((0.0, 0.0), (bg_width as f64, bg_height as f64));
            scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &rect);
        }

        // Clear thread-local visited set for cycle detection
        RENDER_VISITED.with(|visited| {
            let mut visited = visited.borrow_mut();
            visited.clear();

            // Render the root element
            self.render_element(
                scene,
                root_id,
                Point {
                    x: -viewport_scroll.x,
                    y: -viewport_scroll.y,
                },
                &mut visited,
            );
        });

        // Render debug overlay
        if self.devtools.highlight_hover {
            if let Some(node_id) = self.dom.as_ref().get_hover_node_id() {
                render_debug_overlay(scene, self.dom, node_id, self.scale);
            }
        }
    }

    /// Check if screenshot engine is available and active
    ///
    /// Returns true if a screenshot engine is configured and available for processing.
    #[inline]
    pub fn has_screenshot_engine(&self) -> bool {
        self.screenshot_engine.is_some()
    }

    /// Get screenshot engine statistics
    ///
    /// Returns current screenshot engine statistics for monitoring and debugging.
    /// Returns None if no screenshot engine is configured.
    #[inline]
    pub fn screenshot_stats(&self) -> Option<&crate::screenshot::ScreenshotStats> {
        self.screenshot_engine.as_ref().map(|engine| engine.stats())
    }

    /// Process screenshot requests after frame rendering completes
    ///
    /// This method should be called by the graphics backend that has mutable access
    /// to the ScreenshotEngine and the rendered WGPU texture/texture_view.
    ///
    /// Example usage pattern:
    /// ```rust,ignore
    /// // In graphics backend after paint_scene() completes:
    /// painter.paint_scene(&mut scene);
    ///
    /// if painter.has_screenshot_engine() {
    ///     // Get mutable reference to screenshot engine from graphics context
    ///     if let Some(engine) = graphics_context.screenshot_engine_mut() {
    ///         let processed = engine.process_pending_requests(&texture, &texture_view).await?;
    ///     }
    /// }
    /// ```
    ///
    /// This design maintains the integration point while allowing proper mutable access
    /// patterns and avoiding lock-based synchronization as required.
    #[inline]
    pub fn needs_screenshot_processing() -> bool {
        // This is a marker method to indicate where screenshot processing should occur
        // The actual processing happens in the graphics backend with proper engine access
        true
    }

    /// Renders a node, but is guaranteed that the node is an element
    /// This is because the font_size is calculated from layout resolution and all text is rendered directly here, instead
    /// of a separate text stroking phase.
    ///
    /// In Blitz, text styling gets its attributes from its container element/resolved styles
    /// In other libraries, text gets its attributes from a `text` element - this is not how HTML works.
    ///
    /// Approaching rendering this way guarantees we have all the styles we need when rendering text with not having
    /// to traverse back to the parent for its styles, or needing to pass down styles
    fn render_element(
        &self,
        scene: &mut impl PaintScene,
        node_id: usize,
        location: Point,
        visited: &mut HashSet<RenderKey>,
    ) {
        // Cycle detection with state + visited tracking - prevents infinite recursion
        let render_key = make_render_key(node_id, location);
        if visited.contains(&render_key) {
            return; // Cycle detected - same node at same location
        }
        visited.insert(render_key);

        // Skip if we've already rendered this node in the current pass
        {
            let mut state = self.render_state.borrow_mut();
            let was_already_rendered = !state.rendered_nodes.insert(node_id);
            if was_already_rendered {
                visited.remove(&render_key);
                return;
            }
        }
        let node = &self.dom.as_ref().tree()[node_id];

        // Early return if the element is hidden
        if matches!(node.style().display, taffy::Display::None) {
            visited.remove(&render_key);
            return;
        }

        // Only draw elements with a style
        if node.primary_styles().is_none() {
            visited.remove(&render_key);
            return;
        }

        // Hide inputs with type=hidden
        // Implemented here rather than using the style engine for performance reasons
        if node.local_name() == "input" && node.attr(local_name!("type")) == Some("hidden") {
            visited.remove(&render_key);
            return;
        }

        // Hide elements with a visibility style other than visible
        if node
            .primary_styles()
            .unwrap()
            .get_inherited_box()
            .visibility
            != StyloVisibility::Visible
        {
            visited.remove(&render_key);
            return;
        }

        // We can't fully support opacity yet, but we can hide elements with opacity 0
        // Graceful handling: default to fully opaque if styles are missing
        let opacity = node
            .primary_styles()
            .map(|styles| styles.get_effects().opacity)
            .unwrap_or(1.0); // CSS specification default: fully opaque
        if opacity == 0.0 {
            visited.remove(&render_key);
            return;
        }
        let has_opacity = opacity < 1.0;

        // TODO: account for overflow_x vs overflow_y
        // Graceful handling: use default overflow values if styles are missing
        let (overflow_x, overflow_y) = match node.primary_styles() {
            Some(styles) => {
                let box_styles = styles.get_box();
                (box_styles.overflow_x, box_styles.overflow_y)
            }
            None => (Overflow::Visible, Overflow::Visible), // CSS specification defaults
        };
        let is_image = node
            .element_data()
            .and_then(|e| e.raster_image_data())
            .is_some();
        let should_clip = is_image
            || !matches!(overflow_x, Overflow::Visible)
            || !matches!(overflow_y, Overflow::Visible);

        // Apply padding/border offset to inline root
        let (layout, box_position) = self.node_position(node_id, location);
        let taffy::Layout {
            size,
            border,
            padding,
            content_size,
            ..
        } = node.final_layout;
        let scaled_pb = (padding + border).map(f64::from);
        let content_position = kurbo::Point {
            x: box_position.x + scaled_pb.left,
            y: box_position.y + scaled_pb.top,
        };
        let content_box_size = kurbo::Size {
            width: (size.width as f64 - scaled_pb.left - scaled_pb.right) * self.scale,
            height: (size.height as f64 - scaled_pb.top - scaled_pb.bottom) * self.scale,
        };

        // Don't render things that are out of view
        let scaled_y = box_position.y * self.scale;
        let scaled_content_height = content_size.height.max(size.height) as f64 * self.scale;
        if scaled_y > self.height as f64 || scaled_y + scaled_content_height < 0.0 {
            visited.remove(&render_key);
            return;
        }

        // Optimise zero-area (/very small area) clips by not rendering at all
        let clip_area = content_box_size.width * content_box_size.height;
        if should_clip && clip_area < 0.01 {
            visited.remove(&render_key);
            return;
        }

        let mut cx = self.element_cx(node, layout, box_position);
        cx.draw_outline(scene);
        cx.draw_outset_box_shadow(scene);

        // Enhanced background rendering with computed styles
        cx.apply_computed_background_styles(scene);
        cx.draw_background(scene);

        // Enhanced border rendering (integrated into draw_border)
        cx.draw_border(scene);

        // TODO: allow layers with opacity to be unclipped (overflow: visible)
        let wants_layer = should_clip | has_opacity;
        let clip = &cx.frame.padding_box_path();

        maybe_with_layer(scene, wants_layer, opacity, cx.transform, clip, |scene| {
            cx.draw_inset_box_shadow(scene);
            cx.stroke_devtools(scene);

            // Now that background has been drawn, offset pos and cx in order to draw our contents scrolled
            let content_position = Point {
                x: content_position.x - node.scroll_offset.x,
                y: content_position.y - node.scroll_offset.y,
            };
            cx.pos = Point {
                x: cx.pos.x - node.scroll_offset.x,
                y: cx.pos.y - node.scroll_offset.y,
            };
            cx.transform = cx.transform.then_translate(Vec2 {
                x: -node.scroll_offset.x,
                y: -node.scroll_offset.y,
            });
            cx.draw_image(scene);
            #[cfg(feature = "svg")]
            cx.draw_svg(scene);
            cx.draw_canvas(scene);
            cx.draw_input(scene);

            cx.draw_text_input_text(scene, content_position);
            cx.draw_inline_layout(scene, content_position);
            cx.draw_marker(scene, content_position);
            cx.draw_children(scene, visited);
        });

        // Remove from visited set when exiting the function
        visited.remove(&render_key);
    }

    fn render_node(
        &self,
        scene: &mut impl PaintScene,
        node_id: usize,
        location: Point,
        visited: &mut HashSet<RenderKey>,
    ) {
        // Note: Cycle detection is handled by render_element for proper cleanup

        let node = &self.dom.as_ref().tree()[node_id];

        match &node.data {
            NodeData::Element(_) | NodeData::AnonymousBlock(_) => {
                self.render_element(scene, node_id, location, visited)
            }
            NodeData::Text(TextNodeData { .. }) => {
                // Text nodes should never be rendered directly
                // (they should always be rendered as part of an inline layout)
                // unreachable!()
            }
            NodeData::Document => {}
            // NodeData::Doctype => {}
            NodeData::Comment => {} // NodeData::ProcessingInstruction { .. } => {}
        }
        // Note: visited set cleanup is handled by render_element
    }

    fn element_cx<'w>(
        &'w self,
        node: &'w Node,
        layout: Layout,
        box_position: Point,
    ) -> ElementCx<'w> {
        let style = node
            .stylo_element_data
            .borrow()
            .as_ref()
            .map(|element_data| element_data.styles.primary().clone())
            .unwrap_or(
                ComputedValues::initial_values_with_font_override(Font::initial_values()).to_arc(),
            );

        let scale = self.scale;

        // todo: maybe cache this so we don't need to constantly be figuring it out
        // It is quite a bit of math to calculate during render/traverse
        // Also! we can cache the bezpaths themselves, saving us a bunch of work
        let frame = ElementFrame::new(&style, &layout, scale);

        // the bezpaths for every element are (potentially) cached (not yet, tbd)
        // By performing the transform, we prevent the cache from becoming invalid when the page shifts around
        // Zero allocation transform computation using stack-only operations
        let transform = {
            let base_transform = Affine::translate(box_position.to_vec2() * scale);

            // Apply CSS transform property (where transforms are 2d)
            //
            // TODO: Handle hit testing correctly for transformed nodes
            // TODO: Implement nested transforms
            let (t, has_3d) = &style
                .get_box()
                .transform
                .to_transform_3d_matrix(None)
                .unwrap_or((Transform3D::default(), false));

            if !has_3d {
                // Zero allocation transform composition - compute final transform in single expression
                let transform_origin = &style.get_box().transform_origin;
                let origin_x = transform_origin
                    .horizontal
                    .resolve(CSSPixelLength::new(frame.border_box.width() as f32))
                    .px() as f64;
                let origin_y = transform_origin
                    .vertical
                    .resolve(CSSPixelLength::new(frame.border_box.width() as f32))
                    .px() as f64;

                // Single-expression transform composition eliminates intermediate allocations
                // See: https://drafts.csswg.org/css-transforms-2/#two-dimensional-subset
                base_transform
                    * Affine::translate(Vec2 {
                        x: origin_x,
                        y: origin_y,
                    })
                    * Affine::new([t.m11, t.m12, t.m21, t.m22, t.m41, t.m42].map(|v| v as f64))
                    * Affine::translate(Vec2 {
                        x: -origin_x,
                        y: -origin_y,
                    })
            } else {
                base_transform
            }
        };

        let element = node.element_data().unwrap();

        ElementCx {
            context: self,
            frame,
            scale,
            style,
            pos: box_position,
            node,
            element,
            transform,
            #[cfg(feature = "svg")]
            svg: element.svg_data(),
            text_input: element.text_input_data(),
            list_item: element.list_item_data.as_deref(),
            devtools: &self.devtools,
        }
    }
}

fn to_image_quality(image_rendering: ImageRendering) -> peniko::ImageQuality {
    match image_rendering {
        ImageRendering::Auto => peniko::ImageQuality::Medium,
        ImageRendering::CrispEdges => peniko::ImageQuality::Low,
        ImageRendering::Pixelated => peniko::ImageQuality::Low,
    }
}

/// Ensure that the `resized_image` field has a correctly sized image
fn to_peniko_image(image: &RasterImageData, quality: peniko::ImageQuality) -> peniko::Image {
    peniko::Image {
        data: peniko::Blob::new(image.data.clone()),
        format: peniko::ImageFormat::Rgba8,
        width: image.width,
        height: image.height,
        alpha: 1.0,
        x_extend: peniko::Extend::Repeat,
        y_extend: peniko::Extend::Repeat,
        quality,
    }
}

/// Safe border width extraction with comprehensive error handling
/// Prevents overflow, NaN, and infinite values from breaking rendering
#[inline(always)]
fn safe_border_width_px(width_value: f32) -> f32 {
    // Comprehensive validation and clamping for production safety
    if width_value.is_nan() || width_value.is_infinite() {
        DEFAULT_BORDER_WIDTH_PX // Fallback for invalid values
    } else if width_value < 0.0 {
        0.0 // Negative widths are invalid in CSS specification
    } else if width_value > MAX_BORDER_WIDTH_PX {
        MAX_BORDER_WIDTH_PX // Clamp to maximum to prevent overflow
    } else {
        width_value // Valid value - use as-is
    }
}

/// A context of loaded and hot data to draw the element from
struct ElementCx<'a> {
    context: &'a BlitzDomPainter<'a>,
    frame: ElementFrame,
    style: style::servo_arc::Arc<ComputedValues>,
    pos: Point,
    scale: f64,
    node: &'a Node,
    element: &'a ElementData,
    transform: Affine,
    #[cfg(feature = "svg")]
    svg: Option<&'a usvg::Tree>,
    text_input: Option<&'a TextInputData>,
    list_item: Option<&'a ListItemLayout>,
    devtools: &'a DevtoolSettings,
}

impl ElementCx<'_> {
    /// Enhanced background style application with zero allocation
    #[inline(always)]
    fn apply_computed_background_styles(&self, scene: &mut impl PaintScene) {
        // Skip background rendering for input elements - they handle their own backgrounds
        if self.node.local_name() == "input" {
            return;
        }

        let background_styles = self.style.get_background();

        // Only apply enhanced background if no complex background images are specified
        // This avoids duplication with draw_background() which handles images/gradients
        if background_styles
            .background_image
            .0
            .iter()
            .all(|img| matches!(img, GenericImage::None))
        {
            let current_color = self.style.clone_color();

            // Zero-allocation background color extraction
            let bg_color = background_styles
                .background_color
                .resolve_to_absolute(&current_color)
                .as_srgb_color();

            // Enhanced visibility - only render if alpha > epsilon threshold
            if bg_color.components[3] > ALPHA_VISIBILITY_THRESHOLD {
                scene.fill(
                    Fill::NonZero,
                    self.transform,
                    bg_color,
                    None,
                    &self.frame.border_box_path(),
                );
            }
        }
    }

    /// Enhanced text color application with zero allocation
    #[inline(always)]
    #[allow(dead_code)]
    fn apply_computed_text_styles(&self, scene: &mut impl PaintScene, pos: Point) {
        // Apply enhanced text rendering to any inline layout
        if self.node.flags.is_inline_root() {
            if let Some(text_layout) = &self.element.inline_layout_data {
                crate::text::render_text_buffer(
                    self.scale,
                    scene,
                    &text_layout.layout.inner(),
                    pos,
                    Some(&self.style),
                    &blitz_dom::node::TextBrush::from_color(extract_text_color(&self.style)),
                );
            }
        }
    }

    fn draw_inline_layout(&self, scene: &mut impl PaintScene, pos: Point) {
        if self.node.flags.is_inline_root() {
            #[cfg(feature = "tracing")]
            tracing::debug!("draw_inline_layout called for inline root node");

            // Graceful handling: skip rendering if inline layout data is missing
            let Some(text_layout) = self.element.inline_layout_data.as_ref() else {
                // Log warning for debugging but don't crash the application
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Skipping inline layout rendering - missing inline layout data for node"
                );
                return;
            };

            #[cfg(feature = "tracing")]
            tracing::debug!("Found inline layout data, proceeding with text rendering");

            // Enhanced text rendering with computed CSS styles
            crate::text::render_text_buffer(
                self.scale,
                scene,
                &text_layout.layout.inner(),
                pos,
                Some(&self.style),
                &blitz_dom::node::TextBrush::from_color(extract_text_color(&self.style)),
            );
        }
    }

    fn draw_text_input_text(&self, scene: &mut impl PaintScene, pos: Point) {
        #[cfg(feature = "tracing")]
        tracing::trace!("draw_text_input_text called at pos: {:?}", pos);

        // Render the text in text inputs
        if let Some(input_data) = self.text_input {
            #[cfg(feature = "tracing")]
            tracing::trace!("Input data found, rendering text buffer directly");

            // Use the existing text rendering system for input text
            use blitz_text::Edit;

            use crate::text::render_text_buffer;

            // Create a visible black brush for input text using proper constructor
            let black_color = color::AlphaColor::<color::Srgb>::new([0.0, 0.0, 0.0, 1.0]);
            let brush = blitz_dom::node::TextBrush::from_id_and_color(0, black_color);

            // Render directly from the shaped buffer without cloning
            input_data.editor.with_buffer(|buffer| {
                #[cfg(feature = "tracing")]
                tracing::trace!(
                    "Rendering input text with scale: {}, pos: {:?}",
                    self.scale,
                    pos
                );

                render_text_buffer(
                    self.scale,
                    scene,
                    buffer, // Use the original shaped buffer reference
                    pos,
                    Some(&self.style),
                    &brush,
                );

                #[cfg(feature = "tracing")]
                tracing::trace!("render_text_buffer completed for input text");
            });

            if self.node.is_focussed() {
                // Implement selection/cursor rendering with cosmyc-text
                use blitz_text::Edit;

                // Define cursor and selection colors
                let cursor_color = peniko::Color::from_rgb8(0, 0, 0); // Black cursor
                let selection_color = peniko::Color::from_rgba8(0, 120, 215, 128); // Semi-transparent blue

                input_data.editor.with_buffer(|buffer| {
                    // Get selection bounds
                    let selection_bounds = input_data.editor.selection_bounds();

                    // Render selection rectangles
                    if let Some((start, end)) = selection_bounds {
                        for run in buffer.layout_runs() {
                            let line_i = run.line_i;
                            let _line_y = run.line_y;
                            let line_top = run.line_top;
                            let line_height = run.line_height;

                            if line_i >= start.line && line_i <= end.line {
                                let mut selection_rects = Vec::new();

                                for glyph in run.glyphs.iter() {
                                    let cluster = &run.text[glyph.start..glyph.end];
                                    let total = cluster.grapheme_indices(true).count();
                                    let mut c_x = glyph.x;
                                    let c_w = glyph.w / total as f32;

                                    for (i, c) in cluster.grapheme_indices(true) {
                                        let c_start = glyph.start + i;
                                        let c_end = glyph.start + i + c.len();

                                        if (start.line != line_i || c_end > start.index)
                                            && (end.line != line_i || c_start < end.index)
                                        {
                                            let rect = Rect::new(
                                                pos.x + c_x as f64,
                                                pos.y + line_top as f64,
                                                pos.x + c_x as f64 + c_w as f64,
                                                pos.y + line_top as f64 + line_height as f64,
                                            );
                                            selection_rects.push(rect);
                                        }
                                        c_x += c_w;
                                    }
                                }

                                // Handle empty lines in selection
                                if run.glyphs.is_empty() && end.line > line_i {
                                    let rect = Rect::new(
                                        pos.x as f64,
                                        pos.y + line_top as f64,
                                        pos.x + buffer.size().0.unwrap_or(0.0) as f64,
                                        pos.y + line_top as f64 + line_height as f64,
                                    );
                                    selection_rects.push(rect);
                                }

                                // Render all selection rectangles for this line
                                for rect in selection_rects {
                                    scene.fill(
                                        Fill::NonZero,
                                        Affine::IDENTITY,
                                        selection_color,
                                        None,
                                        &rect,
                                    );
                                }
                            }
                        }
                    }

                    // Render cursor
                    if let Some((cursor_x, cursor_y)) = input_data.editor.cursor_position() {
                        // Find cursor height from layout runs
                        let cursor_height = buffer
                            .layout_runs()
                            .find(|run| {
                                // Find the run containing the cursor
                                let cursor = input_data.editor.cursor();
                                run.line_i == cursor.line
                            })
                            .map(|run| run.line_height)
                            .unwrap_or(20.0); // Fallback height

                        let cursor_rect = Rect::new(
                            pos.x + cursor_x as f64,
                            pos.y + cursor_y as f64,
                            pos.x + cursor_x as f64 + 1.0, // 1px wide cursor
                            pos.y + cursor_y as f64 + cursor_height as f64,
                        );

                        scene.fill(
                            Fill::NonZero,
                            Affine::IDENTITY,
                            cursor_color,
                            None,
                            &cursor_rect,
                        );
                    }
                });
            }
        } else {
            #[cfg(feature = "tracing")]
            tracing::trace!("No input data found for text input rendering");
        }
    }

    fn draw_marker(&self, scene: &mut impl PaintScene, pos: Point) {
        if let Some(ListItemLayout {
            marker,
            position: ListItemLayoutPosition::Outside(layout),
        }) = self.list_item
        {
            // Right align and pad the bullet when rendering outside
            let x_padding = match marker {
                Marker::Char(_) => 8.0,
                Marker::String(_) => 0.0,
            };

            // Calculate proper marker positioning using cosmyc-text layout
            let marker_pos = self.calculate_marker_position_cosmyc(layout.inner(), x_padding, pos);

            let pos = marker_pos;

            // Enhanced marker rendering with computed CSS styles
            crate::text::render_text_buffer(
                self.scale,
                scene,
                layout.inner(), // Get inner Buffer from EnhancedBuffer
                pos,
                Some(&self.style),
                &blitz_dom::node::TextBrush::from_color(extract_text_color(&self.style)),
            );
        }
    }

    /// Calculate precise marker positioning using cosmyc-text layout information
    /// Returns proper Point coordinates considering baseline and text metrics
    fn calculate_marker_position_cosmyc(
        &self,
        layout: &blitz_text::Buffer,
        x_padding: f32,
        base_pos: Point,
    ) -> Point {
        // Get the first line's layout to determine baseline positioning
        if let Some(first_run) = layout.layout_runs().next() {
            // Extract line metrics for proper vertical alignment
            let line_height = first_run.line_height;
            let _line_top = first_run.line_top;
            let baseline_y = first_run.line_y;

            // Calculate marker width for proper horizontal positioning
            // Use the first run's text width as reference for alignment
            let marker_width = layout
                .layout_runs()
                .next()
                .map(|run| run.glyphs.iter().map(|g| g.w).sum::<f32>())
                .unwrap_or(20.0); // Fallback width

            // Position marker to the left with proper spacing
            let x_offset = -(marker_width + x_padding + 8.0); // 8.0 is standard marker spacing

            // Align marker vertically with text baseline
            // Center the marker on the baseline for better visual alignment
            let y_offset = baseline_y - (line_height * 0.1); // Slight adjustment for visual balance

            Point {
                x: base_pos.x + x_offset as f64,
                y: base_pos.y + y_offset as f64,
            }
        } else {
            // Fallback positioning when no layout runs are available
            // Use conservative defaults that work with most text sizes
            Point {
                x: base_pos.x - (x_padding as f64 + 30.0), // Standard fallback offset
                y: base_pos.y + 8.0,                       // Reasonable vertical offset
            }
        }
    }

    fn draw_children(&self, scene: &mut impl PaintScene, visited: &mut HashSet<RenderKey>) {
        if let Some(children) = &*self.node.paint_children.borrow() {
            for child_id in children {
                self.render_node(scene, *child_id, self.pos, visited);
            }
        }
    }

    #[cfg(feature = "svg")]
    fn draw_svg(&self, scene: &mut impl PaintScene) {
        use style::properties::generated::longhands::object_fit::computed_value::T as ObjectFit;

        let Some(svg) = self.svg else {
            return;
        };

        let width = self.frame.content_box.width() as u32;
        let height = self.frame.content_box.height() as u32;
        let svg_size = svg.size();

        let x = self.frame.content_box.origin().x;
        let y = self.frame.content_box.origin().y;

        // let object_fit = self.style.clone_object_fit();
        let object_position = self.style.clone_object_position();

        // Apply object-fit algorithm
        let container_size = taffy::Size {
            width: width as f32,
            height: height as f32,
        };
        let object_size = taffy::Size {
            width: svg_size.width(),
            height: svg_size.height(),
        };
        let paint_size = compute_object_fit(container_size, Some(object_size), ObjectFit::Contain);

        // Compute object-position
        let x_offset = object_position.horizontal.resolve(
            CSSPixelLength::new(container_size.width - paint_size.width) / self.scale as f32,
        ) * self.scale as f32;
        let y_offset = object_position.vertical.resolve(
            CSSPixelLength::new(container_size.height - paint_size.height) / self.scale as f32,
        ) * self.scale as f32;
        let x = x + x_offset.px() as f64;
        let y = y + y_offset.px() as f64;

        let x_scale = paint_size.width as f64 / object_size.width as f64;
        let y_scale = paint_size.height as f64 / object_size.height as f64;

        let transform =
            Affine::translate((self.pos.x * self.scale + x, self.pos.y * self.scale + y))
                .pre_scale_non_uniform(x_scale, y_scale);

        anyrender_svg::render_svg_tree(scene, svg, transform);
    }

    fn draw_image(&self, scene: &mut impl PaintScene) {
        if let Some(image) = self.element.raster_image_data() {
            let width = self.frame.content_box.width() as u32;
            let height = self.frame.content_box.height() as u32;
            let x = self.frame.content_box.origin().x;
            let y = self.frame.content_box.origin().y;

            let object_fit = self.style.clone_object_fit();
            let object_position = self.style.clone_object_position();
            let image_rendering = self.style.clone_image_rendering();
            let quality = to_image_quality(image_rendering);

            // Apply object-fit algorithm
            let container_size = taffy::Size {
                width: width as f32,
                height: height as f32,
            };
            let object_size = taffy::Size {
                width: image.width as f32,
                height: image.height as f32,
            };
            let paint_size = compute_object_fit(container_size, Some(object_size), object_fit);

            // Compute object-position
            let x_offset = object_position.horizontal.resolve(
                CSSPixelLength::new(container_size.width - paint_size.width) / self.scale as f32,
            ) * self.scale as f32;
            let y_offset = object_position.vertical.resolve(
                CSSPixelLength::new(container_size.height - paint_size.height) / self.scale as f32,
            ) * self.scale as f32;
            let x = x + x_offset.px() as f64;
            let y = y + y_offset.px() as f64;

            let x_scale = paint_size.width as f64 / object_size.width as f64;
            let y_scale = paint_size.height as f64 / object_size.height as f64;
            let transform = self
                .transform
                .pre_scale_non_uniform(x_scale, y_scale)
                .then_translate(Vec2 { x, y });

            scene.draw_image(&to_peniko_image(image, quality), transform);
        }
    }

    fn draw_canvas(&self, scene: &mut impl PaintScene) {
        if let Some(custom_paint_source) = self.element.canvas_data() {
            let width = self.frame.content_box.width() as u32;
            let height = self.frame.content_box.height() as u32;
            let x = self.frame.content_box.origin().x;
            let y = self.frame.content_box.origin().y;

            if width == 0 || height == 0 {
                return;
            }

            let transform = self.transform.then_translate(Vec2 { x, y });

            scene.fill(
                Fill::NonZero,
                transform,
                // TODO: replace `Arc<dyn Any>` with `CustomPaint` in API?
                Paint::Custom(Arc::new(CustomPaint {
                    source_id: custom_paint_source.custom_paint_source_id,
                    width,
                    height,
                    scale: self.scale,
                })),
                None,
                &Rect::from_origin_size((0.0, 0.0), (width as f64, height as f64)),
            );
        }
    }

    fn stroke_devtools(&self, scene: &mut impl PaintScene) {
        if self.devtools.show_layout {
            let shape = &self.frame.border_box;
            let stroke = Stroke::new(self.scale);

            let stroke_color = match self.node.style().display {
                taffy::Display::Block => Color::new([1.0, 0.0, 0.0, 1.0]),
                taffy::Display::Flex => Color::new([0.0, 1.0, 0.0, 1.0]),
                taffy::Display::Grid => Color::new([0.0, 0.0, 1.0, 1.0]),
                taffy::Display::None => Color::new([0.0, 0.0, 1.0, 1.0]),
            };

            scene.stroke(&stroke, self.transform, stroke_color, None, &shape);
        }
    }

    /// Stroke a border
    ///
    /// The border-style property specifies what kind of border to display.
    ///
    /// The following values are allowed:
    /// ❌ dotted - Defines a dotted border
    /// ❌ dashed - Defines a dashed border
    /// ✅ solid - Defines a solid border
    /// ❌ double - Defines a double border
    /// ❌ groove - Defines a 3D grooved border.
    /// ❌ ridge - Defines a 3D ridged border.
    /// ❌ inset - Defines a 3D inset border.
    /// ❌ outset - Defines a 3D outset border.
    /// ✅ none - Defines no border
    /// ✅ hidden - Defines a hidden border
    ///
    /// The border-style property can have from one to four values (for the top border, right border, bottom border, and the left border).
    fn draw_border(&self, sb: &mut impl PaintScene) {
        for edge in [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left] {
            self.draw_border_edge(sb, edge);
        }
    }

    /// The border-style property specifies what kind of border to display.
    ///
    /// [Border](https://www.w3schools.com/css/css_border.asp)
    ///
    /// The following values are allowed:
    /// - ❌ dotted: Defines a dotted border
    /// - ❌ dashed: Defines a dashed border
    /// - ✅ solid: Defines a solid border
    /// - ❌ double: Defines a double border
    /// - ❌ groove: Defines a 3D grooved border*
    /// - ❌ ridge: Defines a 3D ridged border*
    /// - ❌ inset: Defines a 3D inset border*
    /// - ❌ outset: Defines a 3D outset border*
    /// - ✅ none: Defines no border
    /// - ✅ hidden: Defines a hidden border
    ///
    /// [*] The effect depends on the border-color value
    fn draw_border_edge(&self, sb: &mut impl PaintScene, edge: Edge) {
        let style = &*self.style;
        let border = style.get_border();
        let path = self.frame.border_edge_shape(edge);

        let current_color = style.clone_color();

        // Single-pass border property resolution with zero allocation
        let (color, width) = match edge {
            Edge::Top => (
                &border.border_top_color,
                border.border_top_width.to_f32_px(),
            ),
            Edge::Right => (
                &border.border_right_color,
                border.border_right_width.to_f32_px(),
            ),
            Edge::Bottom => (
                &border.border_bottom_color,
                border.border_bottom_width.to_f32_px(),
            ),
            Edge::Left => (
                &border.border_left_color,
                border.border_left_width.to_f32_px(),
            ),
        };

        // Resolve color and width in one operation
        let color = color.resolve_to_absolute(&current_color).as_srgb_color();
        let width = safe_border_width_px(width);

        // Enhanced border visibility check - width and alpha must both be > threshold
        let alpha = color.components[3];

        if width > 0.0 && alpha > ALPHA_VISIBILITY_THRESHOLD {
            sb.fill(Fill::NonZero, self.transform, color, None, &path);
        }
    }

    /// ❌ dotted - Defines a dotted border
    /// ❌ dashed - Defines a dashed border
    /// ✅ solid - Defines a solid border
    /// ❌ double - Defines a double border
    /// ❌ groove - Defines a 3D grooved border. The effect depends on the border-color value
    /// ❌ ridge - Defines a 3D ridged border. The effect depends on the border-color value
    /// ❌ inset - Defines a 3D inset border. The effect depends on the border-color value
    /// ❌ outset - Defines a 3D outset border. The effect depends on the border-color value
    /// ✅ none - Defines no border
    /// ✅ hidden - Defines a hidden border
    fn draw_outline(&self, scene: &mut impl PaintScene) {
        let outline = self.style.get_outline();

        let current_color = self.style.clone_color();
        let color = outline
            .outline_color
            .resolve_to_absolute(&current_color)
            .as_srgb_color();

        let style = match outline.outline_style {
            OutlineStyle::Auto => return,
            OutlineStyle::BorderStyle(style) => style,
        };

        let path = match style {
            BorderStyle::None | BorderStyle::Hidden => return,
            BorderStyle::Solid => self.frame.outline(),

            // TODO: Implement other border styles
            BorderStyle::Inset
            | BorderStyle::Groove
            | BorderStyle::Outset
            | BorderStyle::Ridge
            | BorderStyle::Dotted
            | BorderStyle::Dashed
            | BorderStyle::Double => self.frame.outline(),
        };

        scene.fill(Fill::NonZero, self.transform, color, None, &path);
    }
}

/// Extract text color from computed styles for TextBrush creation
/// Converts stylo computed color values to color::AlphaColor<color::Srgb> for TextBrush
fn extract_text_color(computed: &ComputedValues) -> color::AlphaColor<color::Srgb> {
    use color::{AlphaColor, Srgb};

    let text_styles = computed.get_inherited_text();
    let color = text_styles.color.as_srgb_color();

    // Convert peniko::Color to palette::AlphaColor<Srgb>
    AlphaColor::<Srgb>::new([
        color.components[0],
        color.components[1],
        color.components[2],
        color.components[3],
    ])
}

impl<'a> std::ops::Deref for ElementCx<'a> {
    type Target = BlitzDomPainter<'a>;
    fn deref(&self) -> &Self::Target {
        self.context
    }
}
