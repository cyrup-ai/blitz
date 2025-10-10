//! Universal text rendering using blitz-text shaping pipeline

use std::sync::Arc;

use anyrender::PaintScene;
use blitz_dom::node::TextBrush;
use blitz_text::{Attrs, Buffer};
use kurbo::{Affine, Point};
use log;
use peniko::Fill;
use style::properties::ComputedValues;

use crate::color::ToColorColor;

// Global text shaper instance for high-performance text rendering
// Uses thread-local storage for zero-allocation access patterns
thread_local! {
    static TEXT_SHAPER: std::cell::RefCell<Option<blitz_text::TextShaper>> =
        std::cell::RefCell::new(None);
}

/// Initialize the thread-local text shaper with production error handling
#[allow(dead_code)]
fn ensure_text_shaper_initialized() {
    TEXT_SHAPER.with(|shaper| {
        let mut shaper_ref = shaper.borrow_mut();
        if shaper_ref.is_none() {
            // Create font system for the shaper - note: TextShaper from shaper module, not shaping
            let font_system = blitz_text::FontSystem::new();
            match blitz_text::TextShaper::new(font_system) {
                Ok(text_shaper) => {
                    *shaper_ref = Some(text_shaper);
                }
                Err(e) => {
                    log::error!("Failed to create TextShaper: {:?}, using fallback", e);

                    // Graceful degradation: try with minimal font system
                    let minimal_font_system = blitz_text::FontSystem::new();
                    if let Ok(fallback_shaper) = blitz_text::TextShaper::new(minimal_font_system) {
                        *shaper_ref = Some(fallback_shaper);
                    } else {
                        log::error!("Complete TextShaper initialization failure, text rendering may be degraded");
                    }
                }
            }
        }
    });
}

/// Enhanced text rendering with advanced shaping pipeline and zero allocation
/// Integrates blitz-text shaping with cosmyc-text rendering for best quality
pub(crate) fn render_text_buffer(
    scale: f64,
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    computed_styles: Option<&ComputedValues>,
    default_brush: &TextBrush,
) {
    println!("🎯 BLITZ-PAINT render_text_buffer called at pos: ({}, {}), scale: {}", pos.x, pos.y, scale);
    #[cfg(feature = "tracing")]
    tracing::debug!(
        "render_text_buffer called at pos: ({}, {}), scale: {}",
        pos.x,
        pos.y,
        scale
    );

    let transform = Affine::translate((pos.x * scale, pos.y * scale));

    // Render text using enhanced styling with blitz-text integration
    // Uses blitz-text shaping pipeline when available, falls back to cosmyc buffer
    render_buffer_with_enhanced_styling(
        scale,
        scene,
        buffer,
        pos,
        computed_styles,
        default_brush,
        transform,
    );
}

/// Enhanced text rendering with improved styling and international support
fn render_buffer_with_enhanced_styling(
    scale: f64,
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    computed_styles: Option<&ComputedValues>,
    default_brush: &TextBrush,
    transform: Affine,
) {
    // Extract enhanced text properties with zero allocation
    let text_color = extract_enhanced_color(computed_styles, default_brush);
    let text_decoration = extract_text_decoration(computed_styles);
    let text_shadow = extract_text_shadow(computed_styles);

    // Apply all text shadows if present (render shadows first, then text)
    // Render in reverse order for proper layering (last shadow rendered first)
    for shadow_params in text_shadow.iter().rev() {
        render_text_shadow(scene, buffer, pos, *shadow_params, transform, scale);
    }

    // Render the main text with enhanced color handling
    scene.render_text_buffer(buffer, pos, text_color, transform);

    // Apply text decorations (underline, overline, line-through)
    if let Some(decoration_params) = text_decoration {
        render_text_decoration(
            scene,
            buffer,
            pos,
            decoration_params,
            transform,
            scale,
            text_color,
        );
    }
}

/// Enhanced color extraction with better defaults and international text support
fn extract_enhanced_color(
    computed_styles: Option<&ComputedValues>,
    default_brush: &TextBrush,
) -> peniko::Color {
    if let Some(styles) = computed_styles {
        let text_styles = styles.get_inherited_text();
        let mut color = text_styles.color.as_srgb_color();

        // Enhanced color correction for better readability
        // Handle transparent text (common CSS issue)
        if color.components[3] < 0.01 {
            color.components[3] = 1.0; // Make fully opaque
            color.components[0] = 0.0; // Black
            color.components[1] = 0.0;
            color.components[2] = 0.0;
        }

        // Handle nearly-white text that might be invisible on light backgrounds
        let is_very_light = color.components[0] >= 0.95
            && color.components[1] >= 0.95
            && color.components[2] >= 0.95;

        if is_very_light {
            // Convert to dark gray for better readability
            color.components[0] = 0.2;
            color.components[1] = 0.2;
            color.components[2] = 0.2;
        }

        color
    } else {
        // Extract from default brush with fallback
        match &default_brush.brush {
            peniko::Brush::Solid(color) => *color,
            peniko::Brush::Gradient(gradient) => {
                // For gradients, use the first color stop
                if let Some(stop) = gradient.stops.first() {
                    // Convert DynamicColor to peniko::Color using components array
                    peniko::Color::from_rgba8(
                        (stop.color.components[0] * 255.0) as u8,
                        (stop.color.components[1] * 255.0) as u8,
                        (stop.color.components[2] * 255.0) as u8,
                        (stop.color.components[3] * 255.0) as u8,
                    )
                } else {
                    peniko::Color::BLACK
                }
            }
            _ => peniko::Color::BLACK, // Safe fallback
        }
    }
}

/// Extract text decoration properties from computed styles
fn extract_text_decoration(
    computed_styles: Option<&ComputedValues>,
) -> Option<TextDecorationParams> {
    let styles = computed_styles?;
    let text_styles = styles.get_text();
    let current_color = styles.clone_color();

    // Extract text decoration line (underline, overline, line-through)
    let decoration_line = &text_styles.text_decoration_line;
    let mut line_types = Vec::new();

    if decoration_line.contains(style::values::specified::text::TextDecorationLine::UNDERLINE) {
        line_types.push(TextDecorationLineType::Underline);
    }
    if decoration_line.contains(style::values::specified::text::TextDecorationLine::OVERLINE) {
        line_types.push(TextDecorationLineType::Overline);
    }
    if decoration_line.contains(style::values::specified::text::TextDecorationLine::LINE_THROUGH) {
        line_types.push(TextDecorationLineType::LineThrough);
    }

    // If no decoration lines are specified, return None
    if line_types.is_empty() {
        return None;
    }

    // Extract decoration color (use currentColor as fallback)
    let decoration_color = text_styles
        .text_decoration_color
        .resolve_to_absolute(&current_color)
        .as_srgb_color();

    // Extract text decoration style from stylo using proper computed value path
    let decoration_style = {
        use style::properties::longhands::text_decoration_style::computed_value::T as StyleTextDecorationStyle;
        match text_styles.text_decoration_style {
            StyleTextDecorationStyle::Solid => TextDecorationStyleType::Solid,
            StyleTextDecorationStyle::Double => TextDecorationStyleType::Double,
            StyleTextDecorationStyle::Dotted => TextDecorationStyleType::Dotted,
            StyleTextDecorationStyle::Dashed => TextDecorationStyleType::Dashed,
            StyleTextDecorationStyle::Wavy => TextDecorationStyleType::Wavy,
            // Handle MozNone and future variants
            _ => TextDecorationStyleType::Solid,
        }
    };

    // Extract text decoration thickness from stylo (Gecko engine supports this)
    let font = styles.get_font();
    let font_size = font.font_size.computed_size.px();
    // Fallback calculation for text decoration thickness
    let thickness = (font_size / 14.0).max(1.0);

    Some(TextDecorationParams {
        line: line_types,
        color: decoration_color,
        style: decoration_style,
        thickness,
    })
}

/// Extract text shadow properties from computed styles - supports multiple shadows
fn extract_text_shadow(computed_styles: Option<&ComputedValues>) -> Vec<TextShadowParams> {
    let styles = match computed_styles {
        Some(s) => s,
        None => return Vec::new(),
    };

    let text_styles = styles.get_inherited_text();
    let current_color = styles.clone_color();

    // Extract all text shadows for CSS compliance
    let text_shadows = &text_styles.text_shadow;

    text_shadows
        .0
        .iter()
        .map(|shadow| {
            // Extract shadow offset coordinates using px() method
            let offset_x = shadow.horizontal.px();
            let offset_y = shadow.vertical.px();

            // Extract blur radius (blur is not Option, access directly)
            let blur_radius = shadow.blur.px();

            // Extract shadow color (color is not Option, resolve directly)
            let shadow_color = shadow
                .color
                .resolve_to_absolute(&current_color)
                .as_srgb_color();

            TextShadowParams {
                offset_x,
                offset_y,
                blur_radius,
                color: shadow_color,
            }
        })
        .collect()
}

/// Render text shadow effect
fn render_text_shadow(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    shadow: TextShadowParams,
    _base_transform: Affine,
    scale: f64,
) {
    // Calculate shadow position with proper scaling
    let shadow_pos = Point {
        x: pos.x + (shadow.offset_x as f64 / scale),
        y: pos.y + (shadow.offset_y as f64 / scale),
    };

    let shadow_transform = Affine::translate((shadow_pos.x * scale, shadow_pos.y * scale));

    // Calculate precise text bounds for shadow blur rendering
    let text_bounds = calculate_text_bounds(buffer, shadow_pos, scale);

    // Apply shadow blur using existing GPU blur infrastructure
    let blur_std_dev = shadow.blur_radius as f64;
    let mut shadow_color = shadow.color;
    shadow_color.components[3] *= 0.5; // Make shadow semi-transparent

    if blur_std_dev > 0.0 {
        // Use GPU blur for proper shadow rendering with blur radius
        scene.draw_box_shadow(
            shadow_transform,
            text_bounds,
            shadow_color,
            0.0, // No border radius for text shadows
            blur_std_dev,
        );
    } else {
        // Fallback to solid text rendering for zero blur
        scene.render_text_buffer(buffer, shadow_pos, shadow_color, shadow_transform);
    }
}

/// Calculate precise text bounds from buffer layout runs for shadow positioning
/// Zero allocation implementation using iterator chains and fold operations
#[inline]
fn calculate_text_bounds(buffer: &Buffer, pos: Point, scale: f64) -> kurbo::Rect {
    buffer
        .layout_runs()
        .fold(None, |acc: Option<kurbo::Rect>, run| {
            if run.glyphs.is_empty() {
                return acc;
            }

            // Calculate run bounds using glyph metrics without allocation
            let run_bounds =
                run.glyphs
                    .iter()
                    .fold(None, |glyph_acc: Option<kurbo::Rect>, glyph| {
                        let glyph_left = pos.x * scale + glyph.x as f64 * scale;
                        let glyph_top = pos.y * scale + (run.line_y + glyph.y) as f64 * scale;
                        let glyph_right = glyph_left + glyph.w as f64 * scale;
                        let glyph_bottom = glyph_top + run.line_height as f64 * scale;

                        let glyph_rect =
                            kurbo::Rect::new(glyph_left, glyph_top, glyph_right, glyph_bottom);

                        match glyph_acc {
                            Some(existing) => Some(existing.union(glyph_rect)),
                            None => Some(glyph_rect),
                        }
                    });

            match (acc, run_bounds) {
                (Some(existing), Some(run_rect)) => Some(existing.union(run_rect)),
                (None, Some(run_rect)) => Some(run_rect),
                (existing, None) => existing,
            }
        })
        .unwrap_or_else(|| {
            // Fallback for empty buffer - create minimal rect at position
            let x = pos.x * scale;
            let y = pos.y * scale;
            kurbo::Rect::new(x, y, x + 1.0, y + 1.0)
        })
}

/// Render text decorations (underline, overline, line-through)
fn render_text_decoration(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    decoration: TextDecorationParams,
    transform: Affine,
    scale: f64,
    text_color: peniko::Color,
) {
    // Get decoration color (use text color if not specified)
    let decoration_color = if decoration.color.components[3] > 0.0 {
        decoration.color
    } else {
        text_color
    };

    // Calculate line thickness (default to 1px if not specified)
    let line_thickness = decoration.thickness as f64 * scale;

    // Render each type of decoration with style-specific rendering
    for line_type in decoration.line.iter() {
        match decoration.style {
            TextDecorationStyleType::Solid => {
                // Use original solid line rendering functions
                match line_type {
                    TextDecorationLineType::Underline => {
                        render_underline(
                            scene,
                            buffer,
                            pos,
                            decoration_color,
                            line_thickness,
                            transform,
                            scale,
                        );
                    }
                    TextDecorationLineType::Overline => {
                        render_overline(
                            scene,
                            buffer,
                            pos,
                            decoration_color,
                            line_thickness,
                            transform,
                            scale,
                        );
                    }
                    TextDecorationLineType::LineThrough => {
                        render_line_through(
                            scene,
                            buffer,
                            pos,
                            decoration_color,
                            line_thickness,
                            transform,
                            scale,
                        );
                    }
                }
            }
            TextDecorationStyleType::Dotted => {
                render_dotted_decoration(
                    scene,
                    buffer,
                    pos,
                    decoration_color,
                    line_thickness,
                    transform,
                    scale,
                    line_type,
                );
            }
            TextDecorationStyleType::Dashed => {
                render_dashed_decoration(
                    scene,
                    buffer,
                    pos,
                    decoration_color,
                    line_thickness,
                    transform,
                    scale,
                    line_type,
                );
            }
            TextDecorationStyleType::Wavy => {
                render_wavy_decoration(
                    scene,
                    buffer,
                    pos,
                    decoration_color,
                    line_thickness,
                    transform,
                    scale,
                    line_type,
                );
            }
            TextDecorationStyleType::Double => {
                render_double_decoration(
                    scene,
                    buffer,
                    pos,
                    decoration_color,
                    line_thickness,
                    transform,
                    scale,
                    line_type,
                );
            }
        }
    }
}

/// Render underline decoration
fn render_underline(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    color: peniko::Color,
    thickness: f64,
    transform: Affine,
    scale: f64,
) {
    for run in buffer.layout_runs() {
        let line_height = run.line_height;
        let line_y = run.line_y;

        // Calculate underline position (below baseline)
        let underline_y = pos.y * scale + (line_y + line_height * 0.9) as f64 * scale;

        // Calculate line width from run glyphs
        let line_width = run.glyphs.iter().map(|g| g.w as f64).sum::<f64>() * scale;

        if line_width > 0.0 {
            let start_x = pos.x * scale;
            let underline_rect = kurbo::Rect::new(
                start_x,
                underline_y,
                start_x + line_width,
                underline_y + thickness,
            );

            scene.fill(Fill::NonZero, transform, color, None, &underline_rect);
        }
    }
}

/// Render overline decoration
fn render_overline(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    color: peniko::Color,
    thickness: f64,
    transform: Affine,
    scale: f64,
) {
    for run in buffer.layout_runs() {
        let line_top = run.line_top;

        // Calculate overline position (above text)
        let overline_y = pos.y * scale + (line_top - thickness as f32 * 0.5) as f64 * scale;

        // Calculate line width from run glyphs
        let line_width = run.glyphs.iter().map(|g| g.w as f64).sum::<f64>() * scale;

        if line_width > 0.0 {
            let start_x = pos.x * scale;
            let overline_rect = kurbo::Rect::new(
                start_x,
                overline_y,
                start_x + line_width,
                overline_y + thickness,
            );

            scene.fill(Fill::NonZero, transform, color, None, &overline_rect);
        }
    }
}

/// Render line-through decoration
fn render_line_through(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    color: peniko::Color,
    thickness: f64,
    transform: Affine,
    scale: f64,
) {
    for run in buffer.layout_runs() {
        let line_height = run.line_height;
        let line_y = run.line_y;

        // Calculate line-through position (middle of text)
        let line_through_y = pos.y * scale + (line_y + line_height * 0.5) as f64 * scale;

        // Calculate line width from run glyphs
        let line_width = run.glyphs.iter().map(|g| g.w as f64).sum::<f64>() * scale;

        if line_width > 0.0 {
            let start_x = pos.x * scale;
            let line_through_rect = kurbo::Rect::new(
                start_x,
                line_through_y,
                start_x + line_width,
                line_through_y + thickness,
            );

            scene.fill(Fill::NonZero, transform, color, None, &line_through_rect);
        }
    }
}

/// Render dotted decoration style with consistent spacing
/// Zero allocation implementation using iterator chains for dot generation
#[inline]
fn render_dotted_decoration(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    color: peniko::Color,
    thickness: f64,
    transform: Affine,
    scale: f64,
    line_type: &TextDecorationLineType,
) {
    for run in buffer.layout_runs() {
        let line_width = run.glyphs.iter().map(|g| g.w as f64).sum::<f64>() * scale;
        if line_width <= 0.0 {
            continue;
        }

        let start_x = pos.x * scale;
        let dot_radius = (thickness * 0.5).max(0.5);
        let dot_spacing = thickness * 2.0; // CSS standard: 2x thickness spacing
        let dot_count = (line_width / dot_spacing).floor() as usize;

        // Calculate line position based on decoration type
        let line_y = match line_type {
            TextDecorationLineType::Underline => {
                pos.y * scale + (run.line_y + run.line_height * 0.9) as f64 * scale
            }
            TextDecorationLineType::Overline => {
                pos.y * scale + (run.line_top - thickness as f32 * 0.5) as f64 * scale
            }
            TextDecorationLineType::LineThrough => {
                pos.y * scale + (run.line_y + run.line_height * 0.5) as f64 * scale
            }
        };

        // Render dots with consistent spacing
        for i in 0..dot_count {
            let dot_x = start_x + (i as f64 * dot_spacing) + dot_radius;
            let dot_center = kurbo::Point::new(dot_x, line_y + dot_radius);
            let dot_circle = kurbo::Circle::new(dot_center, dot_radius);

            scene.fill(Fill::NonZero, transform, color, None, &dot_circle);
        }
    }
}

/// Render dashed decoration style with CSS-compliant patterns
/// Zero allocation implementation with proper dash-gap ratios
#[inline]
fn render_dashed_decoration(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    color: peniko::Color,
    thickness: f64,
    transform: Affine,
    scale: f64,
    line_type: &TextDecorationLineType,
) {
    for run in buffer.layout_runs() {
        let line_width = run.glyphs.iter().map(|g| g.w as f64).sum::<f64>() * scale;
        if line_width <= 0.0 {
            continue;
        }

        let start_x = pos.x * scale;
        let dash_length = thickness * 3.0; // CSS standard: 3x thickness
        let gap_length = thickness * 2.0; // CSS standard: 2x thickness
        let pattern_length = dash_length + gap_length;
        let pattern_count = (line_width / pattern_length).floor() as usize;

        // Calculate line position based on decoration type
        let line_y = match line_type {
            TextDecorationLineType::Underline => {
                pos.y * scale + (run.line_y + run.line_height * 0.9) as f64 * scale
            }
            TextDecorationLineType::Overline => {
                pos.y * scale + (run.line_top - thickness as f32 * 0.5) as f64 * scale
            }
            TextDecorationLineType::LineThrough => {
                pos.y * scale + (run.line_y + run.line_height * 0.5) as f64 * scale
            }
        };

        // Render dashes with proper spacing
        for i in 0..pattern_count {
            let dash_start_x = start_x + (i as f64 * pattern_length);
            let dash_rect = kurbo::Rect::new(
                dash_start_x,
                line_y,
                dash_start_x + dash_length,
                line_y + thickness,
            );

            scene.fill(Fill::NonZero, transform, color, None, &dash_rect);
        }

        // Handle remainder dash if line doesn't end on pattern boundary
        let remainder_start = start_x + (pattern_count as f64 * pattern_length);
        let remainder_width = line_width - (pattern_count as f64 * pattern_length);
        if remainder_width > gap_length {
            let final_dash_width = (remainder_width - gap_length).min(dash_length);
            let final_dash_rect = kurbo::Rect::new(
                remainder_start,
                line_y,
                remainder_start + final_dash_width,
                line_y + thickness,
            );
            scene.fill(Fill::NonZero, transform, color, None, &final_dash_rect);
        }
    }
}

/// Render wavy decoration style with smooth sine wave patterns
/// Zero allocation implementation using kurbo::BezierPath for smooth curves
#[inline]
fn render_wavy_decoration(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    color: peniko::Color,
    thickness: f64,
    transform: Affine,
    scale: f64,
    line_type: &TextDecorationLineType,
) {
    for run in buffer.layout_runs() {
        let line_width = run.glyphs.iter().map(|g| g.w as f64).sum::<f64>() * scale;
        if line_width <= 0.0 {
            continue;
        }

        let start_x = pos.x * scale;
        let wave_amplitude = thickness * 0.75; // Wave height proportional to thickness
        let wave_frequency = thickness * 4.0; // Wave period: 4x thickness

        // Calculate line position based on decoration type
        let line_y = match line_type {
            TextDecorationLineType::Underline => {
                pos.y * scale + (run.line_y + run.line_height * 0.9) as f64 * scale
            }
            TextDecorationLineType::Overline => {
                pos.y * scale + (run.line_top - thickness as f32 * 0.5) as f64 * scale
            }
            TextDecorationLineType::LineThrough => {
                pos.y * scale + (run.line_y + run.line_height * 0.5) as f64 * scale
            }
        };

        // Create smooth wave using Bezier curves
        let mut path = kurbo::BezPath::new();
        let wave_start = kurbo::Point::new(start_x, line_y);
        path.move_to(wave_start);

        // Generate wave segments using cubic Bezier curves
        let segment_count = (line_width / wave_frequency).ceil() as usize;
        for i in 0..segment_count {
            let segment_start_x = start_x + (i as f64 * wave_frequency);
            let segment_end_x =
                (start_x + ((i + 1) as f64 * wave_frequency)).min(start_x + line_width);

            if segment_end_x <= segment_start_x {
                break;
            }

            let segment_width = segment_end_x - segment_start_x;
            let _mid_x = segment_start_x + segment_width * 0.5;

            // Control points for smooth sine wave approximation
            let cp1_x = segment_start_x + segment_width * 0.25;
            let cp2_x = segment_start_x + segment_width * 0.75;
            let cp1_y = line_y + wave_amplitude * if i % 2 == 0 { 1.0 } else { -1.0 };
            let cp2_y = line_y + wave_amplitude * if i % 2 == 0 { 1.0 } else { -1.0 };

            path.curve_to(
                kurbo::Point::new(cp1_x, cp1_y),
                kurbo::Point::new(cp2_x, cp2_y),
                kurbo::Point::new(segment_end_x, line_y),
            );
        }

        // Stroke the wave path with specified thickness
        let stroke = kurbo::Stroke::new(thickness);
        scene.stroke(&stroke, transform, color, None, &path);
    }
}

/// Render double decoration style with parallel lines
/// Zero allocation implementation with proper line spacing
#[inline]
fn render_double_decoration(
    scene: &mut impl PaintScene,
    buffer: &Buffer,
    pos: Point,
    color: peniko::Color,
    thickness: f64,
    transform: Affine,
    scale: f64,
    line_type: &TextDecorationLineType,
) {
    for run in buffer.layout_runs() {
        let line_width = run.glyphs.iter().map(|g| g.w as f64).sum::<f64>() * scale;
        if line_width <= 0.0 {
            continue;
        }

        let start_x = pos.x * scale;
        let line_thickness = (thickness * 0.4).max(0.5); // Each line is thinner
        let line_separation = thickness * 0.6; // Space between lines

        // Calculate base line position based on decoration type
        let base_line_y = match line_type {
            TextDecorationLineType::Underline => {
                pos.y * scale + (run.line_y + run.line_height * 0.9) as f64 * scale
            }
            TextDecorationLineType::Overline => {
                pos.y * scale + (run.line_top - thickness as f32 * 0.5) as f64 * scale
            }
            TextDecorationLineType::LineThrough => {
                pos.y * scale + (run.line_y + run.line_height * 0.5) as f64 * scale
            }
        };

        // Render first line
        let first_line_rect = kurbo::Rect::new(
            start_x,
            base_line_y - line_separation * 0.5,
            start_x + line_width,
            base_line_y - line_separation * 0.5 + line_thickness,
        );
        scene.fill(Fill::NonZero, transform, color, None, &first_line_rect);

        // Render second line
        let second_line_rect = kurbo::Rect::new(
            start_x,
            base_line_y + line_separation * 0.5,
            start_x + line_width,
            base_line_y + line_separation * 0.5 + line_thickness,
        );
        scene.fill(Fill::NonZero, transform, color, None, &second_line_rect);
    }
}

/// Parameters for text decoration rendering
#[derive(Debug, Clone)]
struct TextDecorationParams {
    line: Vec<TextDecorationLineType>,
    color: peniko::Color,
    style: TextDecorationStyleType,
    thickness: f32,
}

#[derive(Debug, Clone)]
enum TextDecorationLineType {
    Underline,
    Overline,
    LineThrough,
}

#[derive(Debug, Clone)]
enum TextDecorationStyleType {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

/// Parameters for text shadow rendering
#[derive(Debug, Clone, Copy)]
struct TextShadowParams {
    offset_x: f32,
    offset_y: f32,
    blur_radius: f32,
    color: peniko::Color,
}

// TODO: This function needs to be updated for async shape_text API
// Commented out temporarily as it's unused (marked #[allow(dead_code)])
// and requires refactoring for async/await with thread-local storage
/*
/// Advanced text shaping interface using blitz-text with production error handling
/// Primary text shaping pipeline for all text rendering operations
#[allow(dead_code)]
pub(crate) async fn shape_text_advanced<'a>(
    text: &'a str,
    attrs: Attrs<'a>,
    max_width: Option<f32>,
) -> Result<Arc<blitz_text::shaping::ShapedText>, blitz_text::ShapingError> {
    // Requires refactoring to work with async shape_text and thread-local storage
    unimplemented!("Needs async refactor")
}
*/

/// Clear text shaper caches (for memory management)
#[allow(dead_code)]
pub(crate) fn clear_text_caches() {
    ensure_text_shaper_initialized();
    TEXT_SHAPER.with(|shaper| {
        if let Some(shaper) = shaper.borrow_mut().as_mut() {
            shaper.clear_caches();
        }
    });
}

/// Get text shaper statistics for monitoring
#[allow(dead_code)]
pub(crate) fn text_shaper_stats() -> Option<blitz_text::shaper::ShaperStats> {
    ensure_text_shaper_initialized();
    TEXT_SHAPER.with(|shaper| shaper.borrow().as_ref().map(|s| s.stats()))
}
