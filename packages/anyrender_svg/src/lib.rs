// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Render an SVG into any impl of [`anyrender::PaintScene`].
//!
//! This currently lacks support for some important SVG features. Known missing features include: masking, filter effects, group backgrounds
//! path shape-rendering, and patterns.

// LINEBENDER LINT SET - lib.rs - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// Internal utility functions use patterns that trigger these lints:
// - missing_docs: pub(crate) conversion utilities are self-documenting
// - shadow_unrelated: type conversion functions reuse variable names for clarity
// - missing_errors_doc: Error types are re-exported and documented in error module
#![allow(missing_docs, clippy::shadow_unrelated, clippy::missing_errors_doc)]
#![cfg_attr(test, allow(unused_crate_dependencies))] // Some dev dependencies are only used in tests

mod error;
mod render;
mod util;

use anyrender::PaintScene;
pub use error::Error;
use kurbo::Affine;
pub use usvg;

/// Append an SVG to an [`anyrender::PaintScene`].
///
/// This will draw a red box over (some) unsupported elements.
pub fn render_svg_str<S: PaintScene>(
    scene: &mut S,
    svg: &str,
    transform: Affine,
) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    render_svg_tree(scene, &tree, transform);
    Ok(())
}

/// Append an SVG to an [`anyrender::PaintScene`] (with custom error handling).
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn render_svg_str_with<S: PaintScene, F: FnMut(&mut S, &usvg::Node)>(
    scene: &mut S,
    svg: &str,
    transform: Affine,
    error_handler: &mut F,
) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    render_svg_tree_with(scene, &tree, transform, error_handler);
    Ok(())
}

/// Append a [`usvg::Tree`] to an [`anyrender::PaintScene`].
///
/// This will draw a red box over (some) unsupported elements.
pub fn render_svg_tree<S: PaintScene>(scene: &mut S, svg: &usvg::Tree, transform: Affine) {
    render_svg_tree_with(scene, svg, transform, &mut util::default_error_handler);
}

/// Append a [`usvg::Tree`] to an [`anyrender::PaintScene`] (with custom error handling).
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn render_svg_tree_with<S: PaintScene, F: FnMut(&mut S, &usvg::Node)>(
    scene: &mut S,
    svg: &usvg::Tree,
    transform: Affine,
    error_handler: &mut F,
) {
    render::render_group(
        scene,
        svg.root(),
        Affine::IDENTITY,
        transform,
        error_handler,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Affine;
    use peniko::Fill;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq)]
    enum DrawCommand {
        Reset,
        PushLayer {
            blend: peniko::BlendMode,
            alpha: f32,
        },
        PopLayer,
        Fill {
            style: Fill,
        },
        Stroke {
            width: f32,
        },
        DrawImage,
    }

    #[derive(Clone)]
    struct MockPaintScene {
        commands: Rc<RefCell<Vec<DrawCommand>>>,
    }

    impl MockPaintScene {
        fn new() -> Self {
            Self {
                commands: Rc::new(RefCell::new(Vec::new())),
            }
        }

        fn commands(&self) -> Vec<DrawCommand> {
            self.commands.borrow().clone()
        }
    }

    impl anyrender::PaintScene for MockPaintScene {
        fn reset(&mut self) {
            self.commands.borrow_mut().clear();
            self.commands.borrow_mut().push(DrawCommand::Reset);
        }

        fn push_layer(
            &mut self,
            blend: impl Into<peniko::BlendMode>,
            alpha: f32,
            _transform: peniko::kurbo::Affine,
            _clip: &impl peniko::kurbo::Shape,
        ) {
            self.commands.borrow_mut().push(DrawCommand::PushLayer {
                blend: blend.into(),
                alpha,
            });
        }

        fn pop_layer(&mut self) {
            self.commands.borrow_mut().push(DrawCommand::PopLayer);
        }

        fn fill<'a>(
            &mut self,
            style: Fill,
            _transform: peniko::kurbo::Affine,
            _brush: impl Into<anyrender::Paint<'a>>,
            _brush_transform: Option<peniko::kurbo::Affine>,
            _shape: &impl peniko::kurbo::Shape,
        ) {
            self.commands.borrow_mut().push(DrawCommand::Fill { style });
        }

        fn stroke<'a>(
            &mut self,
            style: &peniko::kurbo::Stroke,
            _transform: peniko::kurbo::Affine,
            _brush: impl Into<peniko::BrushRef<'a>>,
            _brush_transform: Option<peniko::kurbo::Affine>,
            _shape: &impl peniko::kurbo::Shape,
        ) {
            self.commands
                .borrow_mut()
                .push(DrawCommand::Stroke { width: style.width as f32 });
        }

        fn draw_image(&mut self, _image: &peniko::Image, _transform: peniko::kurbo::Affine) {
            self.commands.borrow_mut().push(DrawCommand::DrawImage);
        }

        fn render_text_buffer(
            &mut self,
            _buffer: &blitz_text::Buffer,
            _position: peniko::kurbo::Point,
            _color: peniko::Color,
            _transform: peniko::kurbo::Affine,
        ) {
            // Not used in SVG rendering
        }

        fn draw_box_shadow(
            &mut self,
            _transform: peniko::kurbo::Affine,
            _rect: peniko::kurbo::Rect,
            _brush: peniko::Color,
            _radius: f64,
            _std_dev: f64,
        ) {
            // Not used in SVG rendering
        }
    }

    // SUBTASK2: Basic Shape Rendering Tests (10+ tests)

    #[test]
    fn test_rect_with_fill() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="10" y="10" width="50" height="50" fill="red"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_circle_with_fill() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="25" fill="blue"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_path_with_fill() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 10 10 L 50 50 L 10 50 Z" fill="green"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_rect_with_stroke() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="10" y="10" width="50" height="50" fill="none" stroke="red" stroke-width="2"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], DrawCommand::Stroke { width: 2.0 }));
    }

    #[test]
    fn test_circle_with_stroke() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="25" fill="none" stroke="blue" stroke-width="3"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], DrawCommand::Stroke { width: 3.0 }));
    }

    #[test]
    fn test_path_with_stroke() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 10 10 L 50 50" fill="none" stroke="black" stroke-width="1"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], DrawCommand::Stroke { width: 1.0 }));
    }

    #[test]
    fn test_fill_and_stroke_order() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="10" y="10" width="50" height="50" fill="red" stroke="blue" stroke-width="2"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 2);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
        assert!(matches!(commands[1], DrawCommand::Stroke { width: 2.0 }));
    }

    #[test]
    fn test_stroke_and_fill_order() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="10" y="10" width="50" height="50" fill="red" stroke="blue" stroke-width="2" paint-order="stroke fill"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 2);
        // With paint-order="stroke fill", stroke comes first
        assert!(matches!(commands[0], DrawCommand::Stroke { width: 2.0 }));
        assert!(matches!(
            commands[1],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_ellipse_with_fill() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <ellipse cx="50" cy="50" rx="30" ry="20" fill="purple"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_line_with_stroke() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <line x1="10" y1="10" x2="90" y2="90" stroke="black" stroke-width="1"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 2);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
        assert!(matches!(commands[1], DrawCommand::Stroke { width: 1.0 }));
    }

    #[test]
    fn test_polyline_with_stroke() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <polyline points="10,10 50,50 90,10" fill="none" stroke="red" stroke-width="2"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], DrawCommand::Stroke { width: 2.0 }));
    }

    #[test]
    fn test_polygon_with_fill() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <polygon points="50,10 90,90 10,90" fill="orange"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_evenodd_fill_rule() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 10 10 L 50 50 L 10 50 Z" fill="green" fill-rule="evenodd"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::EvenOdd
            }
        ));
    }

    // SUBTASK3: Transform and Layer Tests (5+ tests)

    #[test]
    fn test_group_with_opacity() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <g opacity="0.5">
                <rect width="10" height="10"/>
            </g>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 3);
        assert!(matches!(
            commands[0],
            DrawCommand::PushLayer { alpha: 0.5, .. }
        ));
        assert!(matches!(
            commands[1],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
        assert!(matches!(commands[2], DrawCommand::PopLayer));
    }

    #[test]
    fn test_group_with_blend_mode() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <g style="mix-blend-mode: multiply">
                <rect width="10" height="10"/>
            </g>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        // Should have push_layer, fill, pop_layer
        assert!(commands.len() >= 3);
        assert!(matches!(commands[0], DrawCommand::PushLayer { .. }));
        assert!(matches!(commands[commands.len() - 1], DrawCommand::PopLayer));
    }

    #[test]
    fn test_group_with_clip_path() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <defs>
                <clipPath id="clip1">
                    <circle cx="50" cy="50" r="40"/>
                </clipPath>
            </defs>
            <g clip-path="url(#clip1)">
                <rect width="100" height="100" fill="red"/>
            </g>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        // Should have push_layer with clip, fill, pop_layer
        assert!(commands.len() >= 3);
        assert!(matches!(commands[0], DrawCommand::PushLayer { .. }));
        assert!(matches!(commands[commands.len() - 1], DrawCommand::PopLayer));
    }

    #[test]
    fn test_nested_groups() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <g opacity="0.8">
                <g opacity="0.5">
                    <rect width="10" height="10"/>
                </g>
            </g>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        // Should have: push(0.8), push(0.5), fill, pop, pop
        assert_eq!(commands.len(), 5);
        assert!(matches!(
            commands[0],
            DrawCommand::PushLayer { alpha: 0.8, .. }
        ));
        assert!(matches!(
            commands[1],
            DrawCommand::PushLayer { alpha: 0.5, .. }
        ));
        assert!(matches!(
            commands[2],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
        assert!(matches!(commands[3], DrawCommand::PopLayer));
        assert!(matches!(commands[4], DrawCommand::PopLayer));
    }

    #[test]
    fn test_group_with_transform() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <g transform="translate(10, 20)">
                <rect width="10" height="10"/>
            </g>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        // Transform doesn't create layers, just affects the fill command
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_multiple_shapes_in_group() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <g opacity="0.7">
                <rect width="10" height="10"/>
                <circle cx="50" cy="50" r="25"/>
            </g>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        // push_layer, fill (rect), fill (circle), pop_layer
        assert_eq!(commands.len(), 4);
        assert!(matches!(
            commands[0],
            DrawCommand::PushLayer { alpha: 0.7, .. }
        ));
        assert!(matches!(
            commands[1],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
        assert!(matches!(
            commands[2],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
        assert!(matches!(commands[3], DrawCommand::PopLayer));
    }

    // SUBTASK4: Paint Style Tests (3+ tests)

    #[test]
    fn test_solid_color_fill() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="10" y="10" width="50" height="50" fill="#FF0000"/>
        </svg>"##;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_linear_gradient_fill() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <defs>
                <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" style="stop-color:rgb(255,255,0);stop-opacity:1" />
                    <stop offset="100%" style="stop-color:rgb(255,0,0);stop-opacity:1" />
                </linearGradient>
            </defs>
            <rect width="100" height="100" fill="url(#grad1)"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_radial_gradient_fill() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <defs>
                <radialGradient id="grad2" cx="50%" cy="50%" r="50%">
                    <stop offset="0%" style="stop-color:rgb(255,255,255);stop-opacity:1" />
                    <stop offset="100%" style="stop-color:rgb(0,0,255);stop-opacity:1" />
                </radialGradient>
            </defs>
            <circle cx="50" cy="50" r="40" fill="url(#grad2)"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
    }

    #[test]
    fn test_stroke_cap_styles() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <line x1="10" y1="10" x2="90" y2="10" stroke="black" stroke-width="5" stroke-linecap="round"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 2);
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ));
        assert!(matches!(commands[1], DrawCommand::Stroke { width: 5.0 }));
    }

    #[test]
    fn test_stroke_join_styles() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <polyline points="10,10 50,50 90,10" fill="none" stroke="red" stroke-width="3" stroke-linejoin="bevel"/>
        </svg>"#;

        let mut scene = MockPaintScene::new();
        render_svg_str(&mut scene, svg, Affine::IDENTITY).unwrap();

        let commands = scene.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], DrawCommand::Stroke { width: 3.0 }));
    }

    // SUBTASK5: Error Handling Tests (2+ tests)

    #[test]
    fn test_invalid_svg_returns_error() {
        let svg = "<invalid>not xml</invalid>";
        let mut scene = MockPaintScene::new();
        let result = render_svg_str(&mut scene, svg, Affine::IDENTITY);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_error_handler() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <defs>
                <pattern id="pattern1" width="10" height="10" patternUnits="userSpaceOnUse">
                    <circle cx="5" cy="5" r="3" fill="blue"/>
                </pattern>
            </defs>
            <rect fill="url(#pattern1)" width="10" height="10"/>
        </svg>"#;

        let mut error_count = 0;
        let mut error_handler = |_scene: &mut MockPaintScene, _node: &usvg::Node| {
            error_count += 1;
        };

        let mut scene = MockPaintScene::new();
        render_svg_str_with(&mut scene, svg, Affine::IDENTITY, &mut error_handler).unwrap();

        assert_eq!(
            error_count, 1,
            "Error handler should be called for unsupported pattern"
        );
    }

    #[test]
    fn test_rendering_continues_after_error() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <defs>
                <pattern id="pattern1" width="10" height="10" patternUnits="userSpaceOnUse">
                    <circle cx="5" cy="5" r="3" fill="blue"/>
                </pattern>
            </defs>
            <rect fill="url(#pattern1)" width="10" height="10"/>
            <rect fill="red" width="10" height="10"/>
        </svg>"#;

        let mut error_count = 0;
        let mut error_handler = |_scene: &mut MockPaintScene, _node: &usvg::Node| {
            error_count += 1;
        };

        let mut scene = MockPaintScene::new();
        render_svg_str_with(&mut scene, svg, Affine::IDENTITY, &mut error_handler).unwrap();

        let commands = scene.commands();
        // First rect triggers error (pattern unsupported), second rect renders normally
        assert_eq!(error_count, 1, "Error handler should be called once for pattern");
        assert_eq!(commands.len(), 1, "Second rect should render successfully");
        assert!(matches!(
            commands[0],
            DrawCommand::Fill {
                style: Fill::NonZero
            }
        ), "Second rect should produce a fill command");
    }
}
