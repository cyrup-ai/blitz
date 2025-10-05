use std::rc::Rc;

use anyrender::{CustomPaint, Paint, PaintScene};
use glyphon;
use peniko::kurbo::{Affine, Point, Rect, Shape, Stroke};
use peniko::{BlendMode, BrushRef, Color, Fill};
use rustc_hash::FxHashMap;
use vello::Renderer as VelloRenderer;

use crate::{CustomPaintSource, GlyphonState, custom_paint_source::CustomPaintCtx};

// Conversion functions for kurbo version compatibility (0.12.0 to 0.11.3)
fn convert_affine_to_vello(affine: Affine) -> vello::kurbo::Affine {
    let coeffs = affine.as_coeffs();
    vello::kurbo::Affine::new(coeffs)
}

fn convert_stroke_to_vello(stroke: &Stroke) -> vello::kurbo::Stroke {
    let mut vello_stroke = vello::kurbo::Stroke::new(stroke.width)
        .with_caps(match stroke.start_cap {
            peniko::kurbo::Cap::Butt => vello::kurbo::Cap::Butt,
            peniko::kurbo::Cap::Round => vello::kurbo::Cap::Round,
            peniko::kurbo::Cap::Square => vello::kurbo::Cap::Square,
        })
        .with_join(match stroke.join {
            peniko::kurbo::Join::Bevel => vello::kurbo::Join::Bevel,
            peniko::kurbo::Join::Miter => vello::kurbo::Join::Miter,
            peniko::kurbo::Join::Round => vello::kurbo::Join::Round,
        })
        .with_miter_limit(stroke.miter_limit);
    
    // Handle dash patterns with the correct API
    if !stroke.dash_pattern.is_empty() {
        vello_stroke = vello_stroke.with_dashes(stroke.dash_offset, stroke.dash_pattern.as_slice());
    }
    
    vello_stroke
}

fn convert_rect_to_vello(rect: Rect) -> vello::kurbo::Rect {
    vello::kurbo::Rect::new(rect.x0, rect.y0, rect.x1, rect.y1)
}

fn convert_shape_to_vello<S: Shape>(shape: &S) -> vello::kurbo::BezPath {
    let mut path = vello::kurbo::BezPath::new();
    shape.path_elements(0.1).for_each(|element| {
        use peniko::kurbo::PathEl;
        match element {
            PathEl::MoveTo(p) => path.move_to((p.x, p.y)),
            PathEl::LineTo(p) => path.line_to((p.x, p.y)),
            PathEl::QuadTo(p1, p2) => path.quad_to((p1.x, p1.y), (p2.x, p2.y)),
            PathEl::CurveTo(p1, p2, p3) => path.curve_to((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y)),
            PathEl::ClosePath => path.close_path(),
        }
    });
    path
}

pub struct VelloScenePainter<'r> {
    pub renderer: &'r mut VelloRenderer,
    pub custom_paint_sources: &'r mut FxHashMap<u64, Box<dyn CustomPaintSource>>,
    pub inner: vello::Scene,
    pub glyphon_state: Option<&'r mut GlyphonState>,
}

impl VelloScenePainter<'_> {
    pub fn finish(self) -> vello::Scene {
        self.inner
    }

    fn render_custom_source(&mut self, custom_paint: CustomPaint) -> Option<peniko::Image> {
        let CustomPaint {
            source_id,
            width,
            height,
            scale,
        } = custom_paint;

        println!(
            "🎨🔧 render_custom_source: source_id={}, size={}x{}, scale={}",
            source_id,
            width,
            height,
            scale
        );

        // Split borrows to avoid borrow checker conflict
        let VelloScenePainter {
            renderer,
            custom_paint_sources,
            ..
        } = self;

        // Render custom paint source
        println!(
            "🎨🔧 render_custom_source: looking for source_id {} in map with {} sources",
            source_id,
            custom_paint_sources.len()
        );
        for (id, _) in custom_paint_sources.iter() {
            println!("🎨🔧   available source ID: {}", id);
        }
        let source = custom_paint_sources.get_mut(&source_id)?;
        println!("🎨🔧 render_custom_source: found source, calling source.render()");
        let ctx = CustomPaintCtx::new(renderer);
        let texture_handle = source.render(ctx, width, height, scale)?;
        println!(
            "🎨🔧 render_custom_source: got texture handle ID {}, returning dummy image",
            texture_handle.id
        );

        // Return dummy image
        Some(texture_handle.dummy_image())
    }
}

impl PaintScene for VelloScenePainter<'_> {
    fn reset(&mut self) {
        self.inner.reset();
    }

    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        let vello_transform = convert_affine_to_vello(transform);
        let vello_clip = convert_shape_to_vello(clip);
        self.inner.push_layer(blend, alpha, vello_transform, &vello_clip);
    }

    fn pop_layer(&mut self) {
        self.inner.pop_layer();
    }

    fn stroke<'a>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<BrushRef<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let vello_style = convert_stroke_to_vello(style);
        let vello_transform = convert_affine_to_vello(transform);
        let vello_brush_transform = brush_transform.map(convert_affine_to_vello);
        let vello_shape = convert_shape_to_vello(shape);
        self.inner
            .stroke(&vello_style, vello_transform, brush, vello_brush_transform, &vello_shape);
    }

    fn fill<'a>(
        &mut self,
        style: Fill,
        transform: Affine,
        brush: impl Into<Paint<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let paint: Paint<'_> = brush.into();

        let dummy_image: peniko::Image;
        let brush_ref = match paint {
            Paint::Solid(color) => BrushRef::Solid(color),
            Paint::Gradient(gradient) => BrushRef::Gradient(gradient),
            Paint::Image(image) => BrushRef::Image(image),
            Paint::Custom(custom_paint) => {
                log::trace!("VelloScenePainter::fill - received Paint::Custom");
                let Ok(custom_paint) = custom_paint.downcast::<CustomPaint>() else {
                    log::warn!("failed to downcast custom_paint");
                    return;
                };
                log::trace!(
                    "VelloScenePainter::fill - CustomPaint source_id: {}, size: {}x{}",
                    custom_paint.source_id,
                    custom_paint.width,
                    custom_paint.height
                );
                let Some(image) = self.render_custom_source(*custom_paint) else {
                    log::warn!("render_custom_source returned None");
                    return;
                };
                log::trace!("got image from render_custom_source");
                dummy_image = image;
                BrushRef::Image(&dummy_image)
            }
        };

        let vello_transform = convert_affine_to_vello(transform);
        let vello_brush_transform = brush_transform.map(convert_affine_to_vello);
        let vello_shape = convert_shape_to_vello(shape);
        self.inner
            .fill(style, vello_transform, brush_ref, vello_brush_transform, &vello_shape);
    }

    fn draw_box_shadow(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        std_dev: f64,
    ) {
        let vello_transform = convert_affine_to_vello(transform);
        let vello_rect = convert_rect_to_vello(rect);
        self.inner
            .draw_blurred_rounded_rect(vello_transform, vello_rect, brush, radius, std_dev);
    }

    fn render_text_buffer(
        &mut self,
        buffer: &blitz_text::Buffer,
        position: Point,
        color: peniko::Color,
        transform: Affine,
    ) {
        println!("🎯 render_text_buffer called! glyphon_state is: {}", if self.glyphon_state.is_some() { "Some" } else { "None" });
        if let Some(glyphon) = &mut self.glyphon_state {
            // Convert peniko Color to glyphon Color
            let glyphon_color = glyphon::Color::rgba(
                (color.components[0] * 255.0) as u8,
                (color.components[1] * 255.0) as u8,
                (color.components[2] * 255.0) as u8,
                (color.components[3] * 255.0) as u8,
            );

            // Apply transform scaling to position
            let scaled_pos = transform * position;

            // Collect this text buffer for batch GPU rendering
            println!("🎯 ADDING TEXT AREA at ({}, {}) with {} runs", scaled_pos.x, scaled_pos.y, buffer.layout_runs().count());
            glyphon.pending_text_areas.push(crate::PendingTextArea {
                buffer: Rc::new(buffer.clone()),
                left: scaled_pos.x as f32,
                top: scaled_pos.y as f32,
                scale: 1.0, // Scale is already applied in transform
                color: glyphon_color,
                bounds: glyphon::TextBounds::default(),
                z_index: glyphon.pending_text_areas.len() as f32,
            });

            #[cfg(feature = "debug_text")]
            log::debug!(
                "collected text buffer at ({}, {}), color: rgba({},{},{},{})",
                scaled_pos.x,
                scaled_pos.y,
                color.components[0],
                color.components[1],
                color.components[2],
                color.components[3]
            );
        } else {
            log::warn!(
                "render_text_buffer called but glyphon_state is None - text will not render!"
            );
        }
    }
}
