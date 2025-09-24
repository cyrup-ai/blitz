use std::rc::Rc;

use anyrender::{CustomPaint, Paint, PaintScene};
use glyphon;
use kurbo::{Affine, Rect, Shape, Stroke};
use peniko::{BlendMode, BrushRef, Color, Fill};
use rustc_hash::FxHashMap;
use vello::Renderer as VelloRenderer;

use crate::{CustomPaintSource, GlyphonState, custom_paint_source::CustomPaintCtx};

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

        log::trace!(
            "render_custom_source: source_id={}, size={}x{}, scale={}",
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
        log::trace!(
            "render_custom_source: looking for source_id {} in map with {} sources",
            source_id,
            custom_paint_sources.len()
        );
        for (id, _) in custom_paint_sources.iter() {
            log::trace!("  available source ID: {}", id);
        }
        let source = custom_paint_sources.get_mut(&source_id)?;
        log::trace!("render_custom_source: found source, calling render");
        let ctx = CustomPaintCtx::new(renderer);
        let texture_handle = source.render(ctx, width, height, scale)?;
        log::trace!(
            "render_custom_source: got texture handle ID {}, returning dummy image",
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
        self.inner.push_layer(blend, alpha, transform, clip);
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
        self.inner
            .stroke(style, transform, brush, brush_transform, shape);
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

        self.inner
            .fill(style, transform, brush_ref, brush_transform, shape);
    }

    fn draw_box_shadow(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        std_dev: f64,
    ) {
        self.inner
            .draw_blurred_rounded_rect(transform, rect, brush, radius, std_dev);
    }

    fn render_text_buffer(
        &mut self,
        buffer: &blitz_text::Buffer,
        position: kurbo::Point,
        color: peniko::Color,
        transform: Affine,
    ) {
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
