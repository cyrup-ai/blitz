//! Gradient rendering utilities for converting CSS gradients to peniko gradients.

use kurbo::{self, Affine, Point, Rect, Vec2};
use peniko::{self, Gradient};
use style::color::AbsoluteColor;
use style::values::{
    computed::{
        Angle, AngleOrPercentage, CSSPixelLength, Gradient as StyloGradient, LengthPercentage,
        LineDirection, Percentage,
    },
    generics::{
        NonNegative,
        color::GenericColor,
        image::{
            EndingShape, GenericCircle, GenericEllipse, GenericEndingShape, GenericGradient,
            GenericGradientItem, GradientFlags, ShapeExtent,
        },
        position::GenericPosition,
    },
    specified::{
        percentage::ToPercentage,
        position::{HorizontalPositionKeyword, VerticalPositionKeyword},
    },
};
use style_traits::owned_slice::OwnedSlice;

use crate::color::{Color, ToColorColor};

type GradientItem<T> = GenericGradientItem<GenericColor<Percentage>, T>;
type LinearGradient<'a> = (
    &'a LineDirection,
    &'a [GradientItem<LengthPercentage>],
    GradientFlags,
);
type RadialGradient<'a> = (
    &'a EndingShape<NonNegative<CSSPixelLength>, NonNegative<LengthPercentage>>,
    &'a GenericPosition<LengthPercentage, LengthPercentage>,
    &'a OwnedSlice<GenericGradientItem<GenericColor<Percentage>, LengthPercentage>>,
    GradientFlags,
);
type ConicGradient<'a> = (
    &'a Angle,
    &'a GenericPosition<LengthPercentage, LengthPercentage>,
    &'a OwnedSlice<GenericGradientItem<GenericColor<Percentage>, AngleOrPercentage>>,
    GradientFlags,
);

pub(crate) fn to_peniko_gradient(
    gradient: &StyloGradient,
    origin_rect: Rect,
    bounding_box: Rect,
    scale: f64,
    current_color: &AbsoluteColor,
) -> (peniko::Gradient, Option<Affine>) {
    match gradient {
        // https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/linear-gradient
        GenericGradient::Linear {
            direction,
            items,
            flags,
            // compat_mode,
            ..
        } => linear_gradient(
            (direction, items, *flags),
            origin_rect,
            bounding_box,
            scale,
            current_color,
        ),
        GenericGradient::Radial {
            shape,
            position,
            items,
            flags,
            // compat_mode,
            ..
        } => radial_gradient((shape, position, items, *flags), origin_rect, current_color),
        GenericGradient::Conic {
            angle,
            position,
            items,
            flags,
            ..
        } => conic_gradient((angle, position, items, *flags), origin_rect, current_color),
    }
}

fn linear_gradient(
    gradient: LinearGradient,
    rect: Rect,
    bounding_box: Rect,
    scale: f64,
    current_color: &AbsoluteColor,
) -> (peniko::Gradient, Option<Affine>) {
    let (direction, items, flags) = gradient;

    let center = bounding_box.center();
    let (start, end) = match direction {
        LineDirection::Angle(angle) => {
            let angle = -angle.radians64() + std::f64::consts::PI;
            let offset_length =
                rect.width() / 2.0 * angle.sin().abs() + rect.height() / 2.0 * angle.cos().abs();
            let offset_vec = Vec2::new(angle.sin(), angle.cos()) * offset_length;
            (center - offset_vec, center + offset_vec)
        }
        LineDirection::Horizontal(horizontal) => {
            let start = Point::new(rect.x0, rect.y0 + rect.height() / 2.0);
            let end = Point::new(rect.x1, rect.y0 + rect.height() / 2.0);
            match horizontal {
                HorizontalPositionKeyword::Right => (start, end),
                HorizontalPositionKeyword::Left => (end, start),
            }
        }
        LineDirection::Vertical(vertical) => {
            let start = Point::new(rect.x0 + rect.width() / 2.0, rect.y0);
            let end = Point::new(rect.x0 + rect.width() / 2.0, rect.y1);
            match vertical {
                VerticalPositionKeyword::Top => (end, start),
                VerticalPositionKeyword::Bottom => (start, end),
            }
        }
        LineDirection::Corner(horizontal, vertical) => {
            let (start_x, end_x) = match horizontal {
                HorizontalPositionKeyword::Right => (rect.x0, rect.x1),
                HorizontalPositionKeyword::Left => (rect.x1, rect.x0),
            };
            let (start_y, end_y) = match vertical {
                VerticalPositionKeyword::Top => (rect.y1, rect.y0),
                VerticalPositionKeyword::Bottom => (rect.y0, rect.y1),
            };
            (Point::new(start_x, start_y), Point::new(end_x, end_y))
        }
    };

    let gradient_length = CSSPixelLength::new((start.distance(end) / scale) as f32);
    let repeating = flags.contains(GradientFlags::REPEATING);

    let mut gradient = peniko::Gradient::new_linear(start, end).with_extend(if repeating {
        peniko::Extend::Repeat
    } else {
        peniko::Extend::Pad
    });

    let (first_offset, last_offset) = resolve_length_color_stops(
        current_color,
        items,
        gradient_length,
        &mut gradient,
        repeating,
    );
    if repeating && gradient.stops.len() > 1 {
        gradient.kind = peniko::GradientKind::Linear {
            start: start + (end - start) * first_offset as f64,
            end: end + (start - end) * (1.0 - last_offset) as f64,
        };
    }

    (gradient, None)
}

fn radial_gradient(
    gradient: RadialGradient,
    rect: Rect,
    current_color: &AbsoluteColor,
) -> (peniko::Gradient, Option<Affine>) {
    let (shape, position, items, flags) = gradient;
    let repeating = flags.contains(GradientFlags::REPEATING);

    let mut gradient = peniko::Gradient::new_radial((0.0, 0.0), 1.0).with_extend(if repeating {
        peniko::Extend::Repeat
    } else {
        peniko::Extend::Pad
    });

    let (width_px, height_px) = (
        position
            .horizontal
            .resolve(CSSPixelLength::new(rect.width() as f32))
            .px() as f64,
        position
            .vertical
            .resolve(CSSPixelLength::new(rect.height() as f32))
            .px() as f64,
    );

    let gradient_scale: Option<Vec2> = match shape {
        GenericEndingShape::Circle(circle) => {
            let scale = match circle {
                GenericCircle::Extent(extent) => match extent {
                    ShapeExtent::FarthestSide => width_px
                        .max(rect.width() - width_px)
                        .max(height_px.max(rect.height() - height_px)),
                    ShapeExtent::ClosestSide => width_px
                        .min(rect.width() - width_px)
                        .min(height_px.min(rect.height() - height_px)),
                    ShapeExtent::FarthestCorner => {
                        (width_px.max(rect.width() - width_px)
                            + height_px.max(rect.height() - height_px))
                            * 0.5_f64.sqrt()
                    }
                    ShapeExtent::ClosestCorner => {
                        (width_px.min(rect.width() - width_px)
                            + height_px.min(rect.height() - height_px))
                            * 0.5_f64.sqrt()
                    }
                    _ => 0.0,
                },
                GenericCircle::Radius(radius) => radius.0.px() as f64,
            };
            Some(Vec2::new(scale, scale))
        }
        GenericEndingShape::Ellipse(ellipse) => match ellipse {
            GenericEllipse::Extent(extent) => match extent {
                ShapeExtent::FarthestCorner | ShapeExtent::FarthestSide => {
                    let mut scale = Vec2::new(
                        width_px.max(rect.width() - width_px),
                        height_px.max(rect.height() - height_px),
                    );
                    if *extent == ShapeExtent::FarthestCorner {
                        scale *= 2.0_f64.sqrt();
                    }
                    Some(scale)
                }
                ShapeExtent::ClosestCorner | ShapeExtent::ClosestSide => {
                    let mut scale = Vec2::new(
                        width_px.min(rect.width() - width_px),
                        height_px.min(rect.height() - height_px),
                    );
                    if *extent == ShapeExtent::ClosestCorner {
                        scale *= 2.0_f64.sqrt();
                    }
                    Some(scale)
                }
                _ => None,
            },
            GenericEllipse::Radii(x, y) => Some(Vec2::new(
                x.0.resolve(CSSPixelLength::new(rect.width() as f32)).px() as f64,
                y.0.resolve(CSSPixelLength::new(rect.height() as f32)).px() as f64,
            )),
        },
    };

    let gradient_transform = {
        // If the gradient has no valid scale, we don't need to calculate the color stops
        if let Some(gradient_scale) = gradient_scale {
            let (first_offset, last_offset) = resolve_length_color_stops(
                current_color,
                items,
                CSSPixelLength::new(gradient_scale.x as f32),
                &mut gradient,
                repeating,
            );
            let scale = if repeating && gradient.stops.len() >= 2 {
                (last_offset - first_offset) as f64
            } else {
                1.0
            };
            Some(
                Affine::scale_non_uniform(gradient_scale.x * scale, gradient_scale.y * scale)
                    .then_translate(get_translation(position, rect)),
            )
        } else {
            None
        }
    };

    (gradient, gradient_transform)
}

fn conic_gradient(
    gradient: ConicGradient,
    rect: Rect,
    current_color: &AbsoluteColor,
) -> (peniko::Gradient, Option<Affine>) {
    let (angle, position, items, flags) = gradient;

    let repeating = flags.contains(GradientFlags::REPEATING);
    let mut gradient = peniko::Gradient::new_sweep((0.0, 0.0), 0.0, std::f32::consts::PI * 2.0)
        .with_extend(if repeating {
            peniko::Extend::Repeat
        } else {
            peniko::Extend::Pad
        });

    let (first_offset, last_offset) = resolve_angle_color_stops(
        current_color,
        items,
        CSSPixelLength::new(1.0),
        &mut gradient,
        repeating,
    );
    if repeating && gradient.stops.len() >= 2 {
        gradient.kind = peniko::GradientKind::Sweep {
            center: Point::new(0.0, 0.0),
            start_angle: std::f32::consts::PI * 2.0 * first_offset,
            end_angle: std::f32::consts::PI * 2.0 * last_offset,
        };
    }

    let gradient_transform = Some(
        Affine::rotate(angle.radians() as f64 - std::f64::consts::PI / 2.0)
            .then_translate(get_translation(position, rect)),
    );

    (gradient, gradient_transform)
}

#[inline]
fn resolve_length_color_stops(
    current_color: &AbsoluteColor,
    items: &[GradientItem<LengthPercentage>],
    gradient_length: CSSPixelLength,
    gradient: &mut Gradient,
    repeating: bool,
) -> (f32, f32) {
    resolve_color_stops(
        current_color,
        items,
        gradient_length,
        gradient,
        repeating,
        |gradient_length: CSSPixelLength, position: &LengthPercentage| -> Option<f32> {
            position
                .to_percentage_of(gradient_length)
                .map(|percentage| percentage.to_percentage())
        },
    )
}

/// Number of interpolation steps for smooth color transitions
const INTERPOLATION_STEPS: usize = 7;
/// Denominator used in interpolation calculations (1.0 / 13.0 is approximately 0.0769)
const INTERPOLATION_DENOMINATOR: f32 = 13.0;

/// Resolves gradient color stops with optional interpolation hints.
///
/// # Arguments
/// * `current_color` - The current color context for resolving color values
/// * `items` - List of gradient items (color stops and interpolation hints)
/// * `gradient_length` - The total length of the gradient
/// * `gradient` - The gradient to populate with color stops
/// * `repeating` - Whether the gradient should repeat
/// * `item_resolver` - Function to resolve gradient item positions
///
/// # Returns
/// A tuple of (first_offset, last_offset) for the gradient stops
#[inline]
fn resolve_color_stops<T>(
    current_color: &AbsoluteColor,
    items: &[GradientItem<T>],
    gradient_length: CSSPixelLength,
    gradient: &mut Gradient,
    repeating: bool,
    item_resolver: impl Fn(CSSPixelLength, &T) -> Option<f32>,
) -> (f32, f32) {
    let mut hint: Option<f32> = None;

    for (idx, item) in items.iter().enumerate() {
        let (color, offset) = match item {
            GenericGradientItem::SimpleColorStop(color) => {
                let step = 1.0 / (items.len() as f32 - 1.0);
                (
                    color.resolve_to_absolute(current_color).as_dynamic_color(),
                    step * idx as f32,
                )
            }
            GenericGradientItem::ComplexColorStop { color, position } => {
                let offset = item_resolver(gradient_length, position);
                if let Some(offset) = offset {
                    (
                        color.resolve_to_absolute(current_color).as_dynamic_color(),
                        offset,
                    )
                } else {
                    continue;
                }
            }
            GenericGradientItem::InterpolationHint(position) => {
                hint = item_resolver(gradient_length, position);
                continue;
            }
        };

        if idx == 0 && !repeating && offset != 0.0 {
            gradient
                .stops
                .push(peniko::ColorStop { color, offset: 0.0 });
        }

        match hint {
            None => gradient.stops.push(peniko::ColorStop { color, offset }),
            Some(hint) => {
                // Safe access with debug assertion - we should never have an empty stops vector here
                // because we add the first stop before processing any hints
                debug_assert!(
                    !gradient.stops.is_empty(),
                    "Gradient stops should not be empty when processing hints"
                );
                let last_stop = if let Some(stop) = gradient.stops.last() {
                    *stop
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Empty gradient stops when processing hint, using fallback");
                    peniko::ColorStop {
                        color: color.clone(),
                        offset: 0.0,
                    }
                };

                if hint <= last_stop.offset {
                    // Upstream code has a bug here, so we're going to do something different
                    match gradient.stops.len() {
                        0 => (),
                        1 => {
                            gradient.stops.pop();
                        }
                        _ => {
                            let prev_stop = gradient.stops[gradient.stops.len() - 2];
                            if prev_stop.offset == hint {
                                gradient.stops.pop();
                            }
                        }
                    }
                    gradient.stops.push(peniko::ColorStop {
                        color,
                        offset: hint,
                    });
                } else if hint >= offset {
                    gradient.stops.push(peniko::ColorStop {
                        color: last_stop.color,
                        offset: hint,
                    });
                    gradient.stops.push(peniko::ColorStop {
                        color,
                        offset: last_stop.offset,
                    });
                } else if hint == (last_stop.offset + offset) / 2.0 {
                    gradient.stops.push(peniko::ColorStop { color, offset });
                } else {
                    let mid_point = (hint - last_stop.offset) / (offset - last_stop.offset);

                    // Define interpolation function as a closure to avoid code duplication
                    let interpolate_color = |t: f32| -> Color {
                        let relative_offset = (t - last_stop.offset) / (offset - last_stop.offset);
                        let multiplier = relative_offset.powf(0.5f32.log(mid_point));
                        let [last_r, last_g, last_b, last_a] = last_stop.color.components;
                        let [r, g, b, a] = color.components;

                        Color::new([
                            (last_r + multiplier * (r - last_r)),
                            (last_g + multiplier * (g - last_g)),
                            (last_b + multiplier * (b - last_b)),
                            (last_a + multiplier * (a - last_a)),
                        ])
                    };

                    // Add interpolated stops based on the mid-point position
                    if mid_point > 0.5 {
                        // More interpolation points before the hint
                        for step in 0..INTERPOLATION_STEPS {
                            let t = last_stop.offset
                                + (hint - last_stop.offset)
                                    * (INTERPOLATION_STEPS as f32 + step as f32)
                                    / INTERPOLATION_DENOMINATOR;
                            gradient.stops.push(peniko::ColorStop {
                                color: interpolate_color(t).into(),
                                offset: t,
                            });
                        }

                        // Add two more points after the hint
                        for &factor in &[1.0 / 3.0, 2.0 / 3.0] {
                            let t = hint + (offset - hint) * factor;
                            gradient.stops.push(peniko::ColorStop {
                                color: interpolate_color(t).into(),
                                offset: t,
                            });
                        }
                    } else {
                        // Add two points before the hint
                        for &factor in &[1.0 / 3.0, 2.0 / 3.0] {
                            let t = last_stop.offset + (hint - last_stop.offset) * factor;
                            gradient.stops.push(peniko::ColorStop {
                                color: interpolate_color(t).into(),
                                offset: t,
                            });
                        }

                        // More interpolation points after the hint
                        for step in 0..INTERPOLATION_STEPS {
                            let t =
                                hint + (offset - hint) * (step as f32) / INTERPOLATION_DENOMINATOR;
                            gradient.stops.push(peniko::ColorStop {
                                color: interpolate_color(t).into(),
                                offset: t,
                            });
                        }
                    }
                    gradient.stops.push(peniko::ColorStop { color, offset });
                }
            }
        }
    }

    // Post-process the stops for repeating gradients
    if repeating && !gradient.stops.is_empty() {
        // Get the first and last stops with proper error handling
        let (first_stop, last_stop) = {
            // Use if-let chains when they stabilize: https://github.com/rust-lang/rust/issues/53667
            match (gradient.stops.first(), gradient.stops.last()) {
                (Some(first), Some(last)) => (first, last),
                // This should never happen since we checked is_empty()
                _ => {
                    #[cfg(debug_assertions)]
                    panic!("Gradient stops are empty after processing");

                    // In release builds, return default values
                    #[cfg(not(debug_assertions))]
                    {
                        return (0.0, 1.0);
                    }
                }
            }
        };

        let first_offset = first_stop.offset;
        let last_offset = last_stop.offset;

        // Only normalize if the stops don't already span the full range
        if (first_offset - 0.0).abs() > f32::EPSILON || (last_offset - 1.0).abs() > f32::EPSILON {
            let range = last_offset - first_offset;
            // Avoid division by zero or very small numbers
            if range > f32::EPSILON {
                let scale_inv = 1.0 / range;
                for stop in &mut *gradient.stops {
                    stop.offset = (stop.offset - first_offset) * scale_inv;
                }
            }
        }

        (first_offset, last_offset)
    } else {
        (0.0, 1.0)
    }
}

#[inline]
fn resolve_angle_color_stops(
    current_color: &AbsoluteColor,
    items: &[GradientItem<AngleOrPercentage>],
    gradient_length: CSSPixelLength,
    gradient: &mut Gradient,
    repeating: bool,
) -> (f32, f32) {
    resolve_color_stops(
        current_color,
        items,
        gradient_length,
        gradient,
        repeating,
        |_gradient_length: CSSPixelLength, position: &AngleOrPercentage| -> Option<f32> {
            match position {
                AngleOrPercentage::Angle(angle) => {
                    Some(angle.radians() / (std::f64::consts::PI * 2.0) as f32)
                }
                AngleOrPercentage::Percentage(percentage) => Some(percentage.to_percentage()),
            }
        },
    )
}

#[inline]
fn get_translation(
    position: &GenericPosition<LengthPercentage, LengthPercentage>,
    rect: Rect,
) -> Vec2 {
    Vec2::new(
        rect.x0
            + position
                .horizontal
                .resolve(CSSPixelLength::new(rect.width() as f32))
                .px() as f64,
        rect.y0
            + position
                .vertical
                .resolve(CSSPixelLength::new(rect.height() as f32))
                .px() as f64,
    )
}
