use anyrender::PaintScene;
use blitz_dom::local_name;
use kurbo::{Affine, BezPath, Cap, Circle, Join, Point, RoundedRect, Stroke, Vec2};
use peniko::Fill;
use style::dom::TElement as _;

use super::ElementCx;
use crate::color::{Color, ToColorColor as _};

impl ElementCx<'_> {
    pub(super) fn draw_input(&self, scene: &mut impl PaintScene) {
        if self.node.local_name() != "input" {
            return;
        }

        let type_attr = self.node.attr(local_name!("type"));

        // Handle checkbox/radio inputs that need checked state
        if matches!(type_attr, Some("checkbox") | Some("radio")) {
            let Some(checked) = self.element.checkbox_input_checked() else {
                return;
            };
            self.draw_checkbox_radio_input(scene, checked, type_attr);
            return;
        }

        // Handle text inputs and other input types
        self.draw_text_input_background(scene, type_attr);
    }

    fn draw_checkbox_radio_input(
        &self,
        scene: &mut impl PaintScene,
        checked: bool,
        type_attr: Option<&str>,
    ) {
        let disabled = self.node.attr(local_name!("disabled")).is_some();

        // TODO this should be coming from css accent-color, but I couldn't find how to retrieve it
        let accent_color = if disabled {
            Color::from_rgba8(209, 209, 209, 255)
        } else {
            self.style.clone_color().as_srgb_color()
        };

        let width = self.frame.border_box.width();
        let height = self.frame.border_box.height();
        let min_dimension = width.min(height);
        let scale = (min_dimension - 4.0).max(0.0) / 16.0;

        let frame = self.frame.border_box.to_rounded_rect(scale * 2.0);

        match type_attr {
            Some("checkbox") => {
                draw_checkbox(scene, checked, frame, self.transform, accent_color, scale);
            }
            Some("radio") => {
                let center = frame.center();
                draw_radio_button(scene, checked, center, self.transform, accent_color, scale);
            }
            _ => {}
        }
    }

    fn draw_text_input_background(&self, scene: &mut impl PaintScene, _type_attr: Option<&str>) {
        let disabled = self.node.attr(local_name!("disabled")).is_some();

        let _width = self.frame.border_box.width();
        let _height = self.frame.border_box.height();

        // Use subtle rounded corners for text inputs
        let frame = self.frame.border_box.to_rounded_rect(2.0);

        // Draw input background
        let input_bg_color = if disabled {
            Color::from_rgba8(245, 245, 245, 255) // Light gray for disabled
        } else {
            Color::WHITE // White background for enabled inputs
        };
        scene.fill(Fill::NonZero, self.transform, input_bg_color, None, &frame);

        // Draw border
        let border_color = if disabled {
            Color::from_rgba8(200, 200, 200, 255)
        } else {
            Color::from_rgba8(118, 118, 118, 255)
        };
        scene.stroke(
            &Stroke::new(1.0),
            self.transform,
            border_color,
            None,
            &frame,
        );
    }
}

fn draw_checkbox(
    scene: &mut impl PaintScene,
    checked: bool,
    frame: RoundedRect,
    transform: Affine,
    accent_color: Color,
    scale: f64,
) {
    if checked {
        scene.fill(Fill::NonZero, transform, accent_color, None, &frame);
        // Tick code derived from masonry
        let mut path = BezPath::new();
        path.move_to((2.0, 9.0));
        path.line_to((6.0, 13.0));
        path.line_to((14.0, 2.0));

        path.apply_affine(Affine::translate(Vec2 { x: 2.0, y: 1.0 }).then_scale(scale));

        let style = Stroke {
            width: 2.0 * scale,
            join: Join::Round,
            miter_limit: 10.0,
            start_cap: Cap::Round,
            end_cap: Cap::Round,
            dash_pattern: Default::default(),
            dash_offset: 0.0,
        };

        scene.stroke(&style, transform, Color::WHITE, None, &path);
    } else {
        scene.fill(Fill::NonZero, transform, Color::WHITE, None, &frame);
        scene.stroke(&Stroke::default(), transform, accent_color, None, &frame);
    }
}

fn draw_radio_button(
    scene: &mut impl PaintScene,
    checked: bool,
    center: Point,
    transform: Affine,
    accent_color: Color,
    scale: f64,
) {
    let outer_ring = Circle::new(center, 8.0 * scale);
    let gap = Circle::new(center, 6.0 * scale);
    let inner_circle = Circle::new(center, 4.0 * scale);
    if checked {
        scene.fill(Fill::NonZero, transform, accent_color, None, &outer_ring);
        scene.fill(Fill::NonZero, transform, Color::WHITE, None, &gap);
        scene.fill(Fill::NonZero, transform, accent_color, None, &inner_circle);
    } else {
        const GRAY: Color = color::palette::css::GRAY;
        scene.fill(Fill::NonZero, transform, GRAY, None, &outer_ring);
        scene.fill(Fill::NonZero, transform, Color::WHITE, None, &gap);
    }
}
