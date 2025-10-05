use std::sync::Arc;

use anyrender::{Paint, PaintScene};
use kurbo::{Affine, Shape};
use peniko::{BlendMode, BrushRef, Color, Fill, Font, color::PremulRgba8};

use crate::vello_cpu::{self, PaintType, Pixmap, RenderMode};

const DEFAULT_TOLERANCE: f64 = 0.1;

// Reverse conversion functions from peniko::kurbo (0.11.3) to kurbo (0.12.0)
fn convert_peniko_affine_to_kurbo(affine: peniko::kurbo::Affine) -> kurbo::Affine {
    let coeffs = affine.as_coeffs();
    kurbo::Affine::new(coeffs)
}

fn convert_peniko_stroke_to_kurbo(stroke: &peniko::kurbo::Stroke) -> kurbo::Stroke {
    kurbo::Stroke::new(stroke.width)
        .with_caps(match stroke.start_cap {
            peniko::kurbo::Cap::Butt => kurbo::Cap::Butt,
            peniko::kurbo::Cap::Round => kurbo::Cap::Round,
            peniko::kurbo::Cap::Square => kurbo::Cap::Square,
        })
        .with_join(match stroke.join {
            peniko::kurbo::Join::Miter => kurbo::Join::Miter,
            peniko::kurbo::Join::Round => kurbo::Join::Round,
            peniko::kurbo::Join::Bevel => kurbo::Join::Bevel,
        })
        .with_miter_limit(stroke.miter_limit)
}

fn convert_peniko_point_to_kurbo(point: peniko::kurbo::Point) -> kurbo::Point {
    kurbo::Point::new(point.x, point.y)
}

fn convert_peniko_rect_to_kurbo(rect: peniko::kurbo::Rect) -> kurbo::Rect {
    kurbo::Rect::new(rect.x0, rect.y0, rect.x1, rect.y1)
}

fn convert_peniko_shape_to_kurbo<S: peniko::kurbo::Shape>(shape: &S) -> kurbo::BezPath {
    let mut path = kurbo::BezPath::new();
    shape.path_elements(DEFAULT_TOLERANCE).for_each(|element| {
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

// Type conversion functions between kurbo 0.12.0 and kurbo 0.11.3
fn convert_affine_to_peniko(affine: kurbo::Affine) -> peniko::kurbo::Affine {
    let coeffs = affine.as_coeffs();
    peniko::kurbo::Affine::new(coeffs)
}

fn convert_stroke_to_peniko(stroke: &kurbo::Stroke) -> peniko::kurbo::Stroke {
    // Basic stroke with width and caps/joins - skip dashes for now to simplify
    peniko::kurbo::Stroke::new(stroke.width)
        .with_caps(match stroke.start_cap {
            kurbo::Cap::Butt => peniko::kurbo::Cap::Butt,
            kurbo::Cap::Round => peniko::kurbo::Cap::Round,
            kurbo::Cap::Square => peniko::kurbo::Cap::Square,
        })
        .with_join(match stroke.join {
            kurbo::Join::Miter => peniko::kurbo::Join::Miter,
            kurbo::Join::Round => peniko::kurbo::Join::Round,
            kurbo::Join::Bevel => peniko::kurbo::Join::Bevel,
        })
        .with_miter_limit(stroke.miter_limit)
}

fn convert_bezpath_to_peniko(path: &kurbo::BezPath) -> peniko::kurbo::BezPath {
    let mut peniko_path = peniko::kurbo::BezPath::new();
    for el in path.elements() {
        match el {
            kurbo::PathEl::MoveTo(p) => peniko_path.move_to((p.x, p.y)),
            kurbo::PathEl::LineTo(p) => peniko_path.line_to((p.x, p.y)),
            kurbo::PathEl::QuadTo(p1, p2) => peniko_path.quad_to((p1.x, p1.y), (p2.x, p2.y)),
            kurbo::PathEl::CurveTo(p1, p2, p3) => peniko_path.curve_to((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y)),
            kurbo::PathEl::ClosePath => peniko_path.close_path(),
        }
    }
    peniko_path
}

fn convert_rect_to_peniko(rect: kurbo::Rect) -> peniko::kurbo::Rect {
    peniko::kurbo::Rect::new(rect.x0, rect.y0, rect.x1, rect.y1)
}

fn brush_ref_to_paint_type<'a>(brush_ref: BrushRef<'a>) -> PaintType {
    match brush_ref {
        BrushRef::Solid(alpha_color) => PaintType::Solid(alpha_color),
        BrushRef::Gradient(gradient) => PaintType::Gradient(gradient.clone()),
        BrushRef::Image(image) => PaintType::Image(vello_cpu::Image {
            pixmap: convert_image(image),
            x_extend: image.x_extend,
            y_extend: image.y_extend,
            quality: image.quality,
        }),
    }
}

fn anyrender_paint_to_vello_cpu_paint<'a>(paint: Paint<'a>) -> PaintType {
    match paint {
        Paint::Solid(alpha_color) => PaintType::Solid(alpha_color),
        Paint::Gradient(gradient) => PaintType::Gradient(gradient.clone()),
        Paint::Image(image) => PaintType::Image(vello_cpu::Image {
            pixmap: convert_image(image),
            x_extend: image.x_extend,
            y_extend: image.y_extend,
            quality: image.quality,
        }),
        // TODO: custom paint
        Paint::Custom(_) => PaintType::Solid(peniko::color::palette::css::TRANSPARENT),
    }
}

#[allow(unused)]
fn convert_image_cached(image: &peniko::Image) -> Arc<Pixmap> {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};
    static CACHE: LazyLock<Mutex<HashMap<u64, Arc<Pixmap>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let mut map = CACHE.lock().unwrap();
    let id = image.data.id();
    let pixmap = map.entry(id).or_insert_with(|| convert_image(image));

    Arc::clone(pixmap)
}

fn convert_image(image: &peniko::Image) -> Arc<Pixmap> {
    Arc::new(Pixmap::from_parts(
        premultiply(image),
        image.width as u16,
        image.height as u16,
    ))
}

fn premultiply(image: &peniko::Image) -> Vec<PremulRgba8> {
    image
        .data
        .as_ref()
        .chunks_exact(4)
        .map(|d| {
            let alpha = d[3] as u16;
            let premultiply = |e: u8| ((e as u16 * alpha) / 255) as u8;
            if alpha == 0 {
                PremulRgba8::from_u8_array([0, 0, 0, 0])
            } else {
                PremulRgba8 {
                    r: premultiply(d[0]),
                    b: premultiply(d[1]),
                    g: premultiply(d[2]),
                    a: d[3],
                }
            }
        })
        .collect()
}

pub struct VelloCpuScenePainter(pub crate::vello_cpu::RenderContext);

impl VelloCpuScenePainter {
    pub fn finish(self) -> Pixmap {
        let mut pixmap = Pixmap::new(self.0.width(), self.0.height());
        self.0
            .render_to_pixmap(&mut pixmap, RenderMode::OptimizeSpeed);
        pixmap
    }
}

impl PaintScene for VelloCpuScenePainter {
    fn reset(&mut self) {
        self.0.reset();
    }

    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: peniko::kurbo::Affine,
        clip: &impl peniko::kurbo::Shape,
    ) {
        let transform = convert_peniko_affine_to_kurbo(transform);
        let clip = convert_peniko_shape_to_kurbo(clip);
        self.0.set_transform(convert_affine_to_peniko(transform));
        self.0.push_layer(
            Some(&convert_bezpath_to_peniko(&clip.into_path(DEFAULT_TOLERANCE))),
            Some(blend.into()),
            Some(alpha),
            None,
        );
    }

    fn pop_layer(&mut self) {
        self.0.pop_layer();
    }

    fn stroke<'a>(
        &mut self,
        style: &peniko::kurbo::Stroke,
        transform: peniko::kurbo::Affine,
        brush: impl Into<BrushRef<'a>>,
        brush_transform: Option<peniko::kurbo::Affine>,
        shape: &impl peniko::kurbo::Shape,
    ) {
        let style = convert_peniko_stroke_to_kurbo(style);
        let transform = convert_peniko_affine_to_kurbo(transform);
        let brush_transform = brush_transform.map(convert_peniko_affine_to_kurbo);
        let shape = convert_peniko_shape_to_kurbo(shape);
        self.0.set_transform(convert_affine_to_peniko(transform));
        self.0.set_stroke(convert_stroke_to_peniko(&style));
        self.0.set_paint(brush_ref_to_paint_type(brush.into()));
        self.0
            .set_paint_transform(convert_affine_to_peniko(brush_transform.unwrap_or(Affine::IDENTITY)));
        self.0.stroke_path(&convert_bezpath_to_peniko(&shape.into_path(DEFAULT_TOLERANCE)));
    }

    fn fill<'a>(
        &mut self,
        style: Fill,
        transform: peniko::kurbo::Affine,
        brush: impl Into<Paint<'a>>,
        brush_transform: Option<peniko::kurbo::Affine>,
        shape: &impl peniko::kurbo::Shape,
    ) {
        let transform = convert_peniko_affine_to_kurbo(transform);
        let brush_transform = brush_transform.map(convert_peniko_affine_to_kurbo);
        let shape = convert_peniko_shape_to_kurbo(shape);
        self.0.set_transform(convert_affine_to_peniko(transform));
        self.0.set_fill_rule(style);
        self.0
            .set_paint(anyrender_paint_to_vello_cpu_paint(brush.into()));
        self.0
            .set_paint_transform(convert_affine_to_peniko(brush_transform.unwrap_or(Affine::IDENTITY)));
        self.0.fill_path(&convert_bezpath_to_peniko(&shape.into_path(DEFAULT_TOLERANCE)));
    }

    fn render_text_buffer(
        &mut self,
        buffer: &blitz_text::Buffer,
        position: peniko::kurbo::Point,
        color: peniko::Color,
        transform: peniko::kurbo::Affine,
    ) {
        let position = convert_peniko_point_to_kurbo(position);
        let transform = convert_peniko_affine_to_kurbo(transform);
        // Set the base transform and paint color
        self.0.set_transform(convert_affine_to_peniko(transform));
        self.0
            .set_paint(brush_ref_to_paint_type(BrushRef::Solid(color)));
        self.0.set_fill_rule(Fill::NonZero);

        // Process each layout run from the blitz_text buffer
        for run in buffer.layout_runs() {
            if run.glyphs.is_empty() {
                continue;
            }

            // Group glyphs by font to minimize glyph run creation overhead
            let mut font_groups: std::collections::BTreeMap<
                blitz_text::fontdb::ID,
                Vec<&blitz_text::LayoutGlyph>,
            > = std::collections::BTreeMap::new();

            for glyph in run.glyphs {
                font_groups.entry(glyph.font_id).or_default().push(glyph);
            }

            // Render each font group
            for (font_id, glyphs) in font_groups {
                if glyphs.is_empty() {
                    continue;
                }

                // Get the first glyph to determine font properties
                let first_glyph = glyphs[0];
                let font_size = first_glyph.font_size;

                // Get actual font data using blitz-text's EnhancedFontSystem
                let font_system = blitz_text::EnhancedFontSystem::new();
                let (font_data, face_index) = font_system.get_font_data_guaranteed(font_id);
                let font_blob = peniko::Blob::new(std::sync::Arc::new(font_data));
                let font = Font::new(font_blob, face_index);

                // Convert blitz_text glyphs to vello_cpu glyphs
                let vello_glyphs: Vec<crate::vello_cpu::vello_common::glyph::Glyph> = glyphs
                    .iter()
                    .map(|layout_glyph| crate::vello_cpu::vello_common::glyph::Glyph {
                        id: layout_glyph.glyph_id as u32,
                        x: position.x as f32 + layout_glyph.x,
                        y: position.y as f32 + run.line_y + layout_glyph.y,
                    })
                    .collect();

                // Render the glyph run with proper font size and positioning
                self.0
                    .glyph_run(&font)
                    .font_size(font_size)
                    .hint(true)
                    .fill_glyphs(vello_glyphs.into_iter());
            }
        }
    }
    fn draw_box_shadow(
        &mut self,
        transform: peniko::kurbo::Affine,
        rect: peniko::kurbo::Rect,
        color: Color,
        radius: f64,
        std_dev: f64,
    ) {
        let transform = convert_peniko_affine_to_kurbo(transform);
        let rect = convert_peniko_rect_to_kurbo(rect);
        self.0.set_transform(convert_affine_to_peniko(transform));
        self.0.set_paint(PaintType::Solid(color));
        self.0
            .fill_blurred_rounded_rect(&convert_rect_to_peniko(rect), radius as f32, std_dev as f32);
    }
}
