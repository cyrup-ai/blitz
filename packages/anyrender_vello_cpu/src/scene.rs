use std::sync::Arc;

use anyrender::{Paint, PaintScene};
use kurbo::{Affine, Rect, Shape, Stroke};
use peniko::{BlendMode, BrushRef, Color, Fill, Font, color::PremulRgba8};

use crate::vello_cpu::{self, PaintType, Pixmap, RenderMode};

const DEFAULT_TOLERANCE: f64 = 0.1;

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
        transform: Affine,
        clip: &impl Shape,
    ) {
        self.0.set_transform(transform);
        self.0.push_layer(
            Some(&clip.into_path(DEFAULT_TOLERANCE)),
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
        style: &Stroke,
        transform: Affine,
        brush: impl Into<BrushRef<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        self.0.set_transform(transform);
        self.0.set_stroke(style.clone());
        self.0.set_paint(brush_ref_to_paint_type(brush.into()));
        self.0
            .set_paint_transform(brush_transform.unwrap_or(Affine::IDENTITY));
        self.0.stroke_path(&shape.into_path(DEFAULT_TOLERANCE));
    }

    fn fill<'a>(
        &mut self,
        style: Fill,
        transform: Affine,
        brush: impl Into<Paint<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        self.0.set_transform(transform);
        self.0.set_fill_rule(style);
        self.0
            .set_paint(anyrender_paint_to_vello_cpu_paint(brush.into()));
        self.0
            .set_paint_transform(brush_transform.unwrap_or(Affine::IDENTITY));
        self.0.fill_path(&shape.into_path(DEFAULT_TOLERANCE));
    }

    fn render_text_buffer(
        &mut self,
        buffer: &blitz_text::Buffer,
        position: kurbo::Point,
        color: peniko::Color,
        transform: Affine,
    ) {
        // Set the base transform and paint color
        self.0.set_transform(transform);
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

                // Get actual font data using blitz-text's FontSystem
                let font_system = blitz_text::FontSystem::new();
                let font = font_system
                    .db()
                    .with_face_data(font_id, |font_data, face_index| {
                        let font_blob = peniko::Blob::new(std::sync::Arc::new(font_data.to_vec()));
                        Font::new(font_blob, face_index)
                    })
                    .unwrap_or_else(|| {
                        // Fallback: create minimal valid font if font ID not found
                        // This should rarely happen in production
                        let fallback_data = vec![0u8; 4];
                        let font_blob = peniko::Blob::new(std::sync::Arc::new(fallback_data));
                        Font::new(font_blob, 0)
                    });

                // Convert blitz_text glyphs to vello_cpu glyphs
                let vello_glyphs: Vec<crate::vello_cpu::Glyph> = glyphs
                    .iter()
                    .map(|layout_glyph| crate::vello_cpu::Glyph {
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
        transform: Affine,
        rect: Rect,
        color: Color,
        radius: f64,
        std_dev: f64,
    ) {
        self.0.set_transform(transform);
        self.0.set_paint(PaintType::Solid(color));
        self.0
            .fill_blurred_rounded_rect(&rect, radius as f32, std_dev as f32);
    }
}
