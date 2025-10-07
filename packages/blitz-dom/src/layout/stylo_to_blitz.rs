// Stylo to Cosmic-Text conversion module
// Zero allocation, blazing fast, no unsafe, no locking
// Converts CSS ComputedValues to cosmyc_text attributes

use blitz_text::{
    AttrsOwned, CacheKeyFlags, Family, FamilyOwned, FontFeatures, Metrics, Stretch,
    Style as FontStyle, Weight, Wrap,
};
use style::properties::ComputedValues;
use style::properties::longhands::list_style_type::computed_value::T as ListStyleType;
use style::properties::longhands::white_space_collapse::computed_value::T as WhiteSpaceCollapse;
use style::values::computed::font::LineHeight;
use style::values::computed::{FontStyle as StyleFontStyle, FontWeight};

/// Convert Stylo ComputedValues to cosmyc_text::Attrs
/// Zero-allocation conversion using references where possible
#[inline(always)]
pub fn style(node_id: usize, computed: &ComputedValues) -> CosmicStyle {
    let font = computed.get_font();
    let text = computed.get_inherited_text();
    let color = text.clone_color();

    // Extract font family with zero allocation
    let family = match font.font_family.families.iter().next() {
        Some(family) => {
            use style::values::computed::font::SingleFontFamily;
            match family {
                SingleFontFamily::Generic(generic) => {
                    use style::values::computed::font::GenericFontFamily;
                    match generic {
                        GenericFontFamily::Serif => Family::Serif,
                        GenericFontFamily::SansSerif => Family::SansSerif,
                        GenericFontFamily::Monospace => Family::Monospace,
                        GenericFontFamily::Cursive => Family::Cursive,
                        GenericFontFamily::Fantasy => Family::Fantasy,
                        _ => Family::SansSerif,
                    }
                }
                SingleFontFamily::FamilyName(name) => Family::Name(name.name.as_ref()),
            }
        }
        _ => Family::SansSerif,
    };

    // Convert font weight - optimized for common cases
    let weight = match font.font_weight {
        FontWeight::NORMAL => Weight::NORMAL,
        FontWeight::BOLD => Weight::BOLD,
        weight => Weight(weight.value() as u16),
    };

    // Convert font style - using stylo's constants
    let style = if font.font_style == StyleFontStyle::NORMAL {
        FontStyle::Normal
    } else if font.font_style == StyleFontStyle::ITALIC {
        FontStyle::Italic
    } else {
        FontStyle::Oblique // For oblique angles
    };

    // Extract font size in pixels
    let font_size_px = font.font_size.computed_size.px();

    // Convert line height efficiently
    let line_height = convert_line_height(&font.line_height, font_size_px);

    // Extract text color using stylo's color space conversion
    let srgb = color.to_color_space(style::color::ColorSpace::Srgb);
    let components = srgb.raw_components();
    let text_color = blitz_text::Color::rgba(
        (components[0].clamp(0.0, 1.0) * 255.0) as u8,
        (components[1].clamp(0.0, 1.0) * 255.0) as u8,
        (components[2].clamp(0.0, 1.0) * 255.0) as u8,
        (components[3].clamp(0.0, 1.0) * 255.0) as u8,
    );

    // Determine text wrap mode from CSS properties
    let wrap = {
        use style::properties::longhands::overflow_wrap::computed_value::T as OverflowWrap;
        use style::properties::longhands::text_wrap_mode::computed_value::T as TextWrapMode;
        use style::properties::longhands::word_break::computed_value::T as WordBreak;

        // white-space has highest priority
        match text.white_space_collapse {
            WhiteSpaceCollapse::Collapse => {
                // Check text-wrap-mode
                match text.text_wrap_mode {
                    TextWrapMode::Wrap => {
                        // Further refined by word-break
                        match text.word_break {
                            WordBreak::BreakAll => Wrap::Glyph,
                            _ => {
                                // Check overflow-wrap for WordOrGlyph
                                match text.overflow_wrap {
                                    OverflowWrap::Anywhere | OverflowWrap::BreakWord => Wrap::WordOrGlyph,
                                    _ => Wrap::Word,
                                }
                            }
                        }
                    }
                    TextWrapMode::Nowrap => Wrap::None,
                    _ => Wrap::Word,
                }
            }
            WhiteSpaceCollapse::Preserve | WhiteSpaceCollapse::PreserveBreaks => {
                // pre and pre-line can still wrap
                match text.text_wrap_mode {
                    TextWrapMode::Wrap => Wrap::Word,
                    _ => Wrap::None,
                }
            }
            WhiteSpaceCollapse::BreakSpaces => Wrap::WordOrGlyph,
        }
    };

    CosmicStyle {
        attrs: AttrsOwned {
            color_opt: Some(text_color),
            family_owned: FamilyOwned::new(family),
            stretch: Stretch::Normal,
            style,
            weight,
            metadata: node_id,
            cache_key_flags: CacheKeyFlags::empty(),
            metrics_opt: None,
            letter_spacing_opt: None,
            font_features: FontFeatures::new(),
        },
        metrics: Metrics {
            font_size: font_size_px,
            line_height,
        },
        wrap,
    }
}

/// Wrapper for cosmyc text style components
pub struct CosmicStyle {
    pub attrs: AttrsOwned,
    pub metrics: Metrics,
    pub wrap: Wrap,
}

impl CosmicStyle {
    /// Create a default style
    pub fn default() -> Self {
        Self {
            attrs: AttrsOwned {
                color_opt: Some(blitz_text::Color::rgb(0, 0, 0)),
                family_owned: FamilyOwned::SansSerif,
                stretch: Stretch::Normal,
                style: FontStyle::Normal,
                weight: Weight::NORMAL,
                metadata: 0,
                cache_key_flags: CacheKeyFlags::empty(),
                metrics_opt: None,
                letter_spacing_opt: None,
                font_features: FontFeatures::new(),
            },
            metrics: Metrics::new(16.0, 19.2),
            wrap: Wrap::Word,
        }
    }
}

/// Convert CSS line-height to absolute pixels
/// Optimized for common cases (normal, relative, absolute)
#[inline(always)]
fn convert_line_height(line_height: &LineHeight, font_size: f32) -> f32 {
    match line_height {
        LineHeight::Normal => font_size * 1.2, // Standard line height multiplier
        LineHeight::Number(n) => font_size * n.0,
        LineHeight::Length(length) => length.px(), // Direct pixel conversion
    }
}

/// Text collapse modes for preprocessing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextCollapseMode {
    /// Collapse whitespace (normal HTML behavior)
    Collapse,
    /// Preserve all whitespace
    Preserve,
    /// Preserve only newlines
    PreserveNewlines,
    /// Preserve whitespace but allow breaking
    PreserveBreakable,
}

/// Convert CSS white-space-collapse property to TextCollapseMode
/// Implements CSS Text Level 4 specification for whitespace processing
#[inline(always)]
pub fn white_space_collapse_to_mode(collapse: WhiteSpaceCollapse) -> TextCollapseMode {
    match collapse {
        WhiteSpaceCollapse::Collapse => TextCollapseMode::Collapse,
        WhiteSpaceCollapse::Preserve => TextCollapseMode::Preserve,
        WhiteSpaceCollapse::PreserveBreaks => TextCollapseMode::PreserveNewlines,
        WhiteSpaceCollapse::BreakSpaces => TextCollapseMode::PreserveBreakable,
    }
}

/// Create font stack for special symbols (like bullets)
/// Returns appropriate font family for rendering special characters
#[inline(always)]
pub fn font_for_bullet(style_type: ListStyleType) -> Option<Family<'static>> {
    match style_type {
        ListStyleType::Disc | ListStyleType::Circle | ListStyleType::Square => {
            // Use Mozilla bullet font for proper bullet rendering
            // Font is loaded in BaseDocument::new() from BULLET_FONT constant
            Some(Family::Name("Moz Bullet Font"))
        }
        _ => None,
    }
}
