use taffy::{BoxSizing, CoreStyle as _, MaybeMath, MaybeResolve, ResolveOrZero as _, Size};

use crate::layout::resolve_calc_value;

#[derive(Debug, Clone, Copy)]
pub struct ReplacedContext {
    pub inherent_size: taffy::Size<f32>,
    pub attr_size: taffy::Size<Option<f32>>,
}

/// Whether a height/width value is violating it's min- and max- constraints
/// The min- and max- constraints cannot both be violated because the max
/// constraint if floored by the min constraint (min constraint takes priority)
enum Violation {
    /// Constraints are not violated
    None,
    /// Min constraint is violated
    Min,
    /// Max constraint is violated
    Max,
}

pub fn replaced_measure_function(
    known_dimensions: taffy::Size<Option<f32>>,
    parent_size: taffy::Size<Option<f32>>,
    image_context: &ReplacedContext,
    style: &taffy::Style,
    _debug: bool,
) -> taffy::Size<f32> {
    let inherent_size = image_context.inherent_size;

    let padding = style
        .padding()
        .resolve_or_zero(parent_size.width, resolve_calc_value);
    let border = style
        .border()
        .resolve_or_zero(parent_size.width, resolve_calc_value);
    let padding_border = padding + border;
    let pb_sum = Size {
        width: padding_border.left + padding_border.right,
        height: padding_border.top + padding_border.bottom,
    };
    let box_sizing_adjustment = if style.box_sizing() == BoxSizing::BorderBox {
        pb_sum
    } else {
        Size::ZERO
    };

    // Use aspect_ratio from style, fall back to inherent aspect ratio
    let s_aspect_ratio = style.aspect_ratio;
    let aspect_ratio = s_aspect_ratio.unwrap_or_else(|| inherent_size.width / inherent_size.height);
    let inv_aspect_ratio = 1.0 / aspect_ratio;

    // Resolve sizes
    let style_size = style
        .size
        .maybe_resolve(parent_size, resolve_calc_value)
        .maybe_apply_aspect_ratio(Some(aspect_ratio))
        .maybe_sub(box_sizing_adjustment);
    let min_size = style
        .min_size
        .maybe_resolve(parent_size, resolve_calc_value)
        .maybe_sub(box_sizing_adjustment);
    let max_size = style
        .max_size
        .maybe_resolve(parent_size, resolve_calc_value)
        .maybe_max(min_size)
        .maybe_sub(box_sizing_adjustment);
    let attr_size = image_context.attr_size;

    let unclamped_size = 'size: {
        if known_dimensions.width.is_some() | known_dimensions.height.is_some() {
            let result = known_dimensions.maybe_apply_aspect_ratio(Some(aspect_ratio));
            match (result.width, result.height) {
                (Some(w), Some(h)) => {
                    break 'size taffy::Size {
                        width: w,
                        height: h,
                    };
                }
                _ => {
                    eprintln!(
                        "Warning: Could not apply aspect ratio to known dimensions, using fallback"
                    );
                    // Continue to next option
                }
            }
        }

        if style_size.width.is_some() | style_size.height.is_some() {
            let result = style_size.maybe_apply_aspect_ratio(Some(aspect_ratio));
            match (result.width, result.height) {
                (Some(w), Some(h)) => {
                    break 'size taffy::Size {
                        width: w,
                        height: h,
                    };
                }
                _ => {
                    eprintln!(
                        "Warning: Could not apply aspect ratio to style size, using fallback"
                    );
                    // Continue to next option
                }
            }
        }

        if attr_size.width.is_some() | attr_size.height.is_some() {
            let result = attr_size.maybe_apply_aspect_ratio(Some(aspect_ratio));
            match (result.width, result.height) {
                (Some(w), Some(h)) => {
                    break 'size taffy::Size {
                        width: w,
                        height: h,
                    };
                }
                _ => {
                    eprintln!("Warning: Could not apply aspect ratio to attr size, using fallback");
                    // Continue to next option
                }
            }
        }

        let result = inherent_size
            .map(Some)
            .maybe_apply_aspect_ratio(Some(aspect_ratio));
        match (result.width, result.height) {
            (Some(w), Some(h)) => taffy::Size {
                width: w,
                height: h,
            },
            _ => {
                eprintln!(
                    "Warning: Could not apply aspect ratio to inherent size, using zero size"
                );
                taffy::Size {
                    width: 0.0,
                    height: 0.0,
                }
            }
        }
    };

    // Floor size at zero
    let size = unclamped_size.map(|s| s.max(0.0));

    // Violations
    let width_violation = if size.width < min_size.width.unwrap_or(0.0) {
        Violation::Min
    } else if size.width > max_size.width.unwrap_or(f32::INFINITY) {
        Violation::Max
    } else {
        Violation::None
    };

    let height_violation = if size.height < min_size.height.unwrap_or(0.0) {
        Violation::Min
    } else if size.height > max_size.height.unwrap_or(f32::INFINITY) {
        Violation::Max
    } else {
        Violation::None
    };

    // Clamp following rules in table at
    // https://www.w3.org/TR/CSS22/visudet.html#min-max-widths
    let size = match (width_violation, height_violation) {
        // No constraint violation
        (Violation::None, Violation::None) => size,
        // w > max-width
        (Violation::Max, Violation::None) => match max_size.width {
            Some(max_width) => Size {
                width: max_width,
                height: (max_width * inv_aspect_ratio).maybe_max(min_size.height),
            },
            None => {
                eprintln!(
                    "Warning: Max width violation detected but no max width constraint found, using original size"
                );
                size
            }
        },
        // w < min-width
        (Violation::Min, Violation::None) => match min_size.width {
            Some(min_width) => Size {
                width: min_width,
                height: (min_width * inv_aspect_ratio).maybe_min(max_size.height),
            },
            None => {
                eprintln!(
                    "Warning: Min width violation detected but no min width constraint found, using original size"
                );
                size
            }
        },
        // h > max-height
        (Violation::None, Violation::Max) => match max_size.height {
            Some(max_height) => Size {
                width: (max_height * aspect_ratio).maybe_max(min_size.width),
                height: max_height,
            },
            None => {
                eprintln!(
                    "Warning: Max height violation detected but no max height constraint found, using original size"
                );
                size
            }
        },
        // h < min-height
        (Violation::None, Violation::Min) => match min_size.height {
            Some(min_height) => Size {
                width: (min_height * aspect_ratio).maybe_min(max_size.width),
                height: min_height,
            },
            None => {
                eprintln!(
                    "Warning: Min height violation detected but no min height constraint found, using original size"
                );
                size
            }
        },
        // (w > max-width) and (h > max-height)
        (Violation::Max, Violation::Max) => match (max_size.width, max_size.height) {
            (Some(max_width), Some(max_height)) => {
                if max_width / size.width <= max_height / size.height {
                    Size {
                        width: max_width,
                        height: (max_width * inv_aspect_ratio).maybe_max(min_size.height),
                    }
                } else {
                    Size {
                        width: (max_height * aspect_ratio).maybe_max(min_size.width),
                        height: max_height,
                    }
                }
            }
            _ => {
                eprintln!(
                    "Warning: Max width/height violation detected but constraints are missing, using original size"
                );
                size
            }
        },
        // (w < min-width) and (h < min-height)
        (Violation::Min, Violation::Min) => match (min_size.width, min_size.height) {
            (Some(min_width), Some(min_height)) => {
                if min_width / size.width <= min_height / size.height {
                    Size {
                        width: (min_height * aspect_ratio).maybe_min(max_size.width),
                        height: min_height,
                    }
                } else {
                    Size {
                        width: min_width,
                        height: (min_width * inv_aspect_ratio).maybe_min(max_size.height),
                    }
                }
            }
            _ => {
                eprintln!(
                    "Warning: Min width/height violation detected but constraints are missing, using original size"
                );
                size
            }
        },
        // (w < min-width) and (h > max-height)
        (Violation::Min, Violation::Max) => match (min_size.width, max_size.height) {
            (Some(min_width), Some(max_height)) => Size {
                width: min_width,
                height: max_height,
            },
            _ => {
                eprintln!(
                    "Warning: Min width/max height violation detected but constraints are missing, using original size"
                );
                size
            }
        },
        // (w > max-width) and (h < min-height)
        (Violation::Max, Violation::Min) => match (max_size.width, min_size.height) {
            (Some(max_width), Some(min_height)) => Size {
                width: max_width,
                height: min_height,
            },
            _ => {
                eprintln!(
                    "Warning: Max width/min height violation detected but constraints are missing, using original size"
                );
                size
            }
        },
    };

    size + pb_sum
}
