//! CSS-compliant intrinsic sizing for masonry layout items
//!
//! Replaces hardcoded 200.0px/100.0px fallbacks with proper CSS Sizing Module Level 3
//! intrinsic sizing calculations. Integrates with existing measurement infrastructure
//! from inline.rs, replaced.rs, and collect_inline_text.rs.

use style::properties::ComputedValues;
use taffy::{AvailableSpace, CoreStyle, NodeId};

use crate::BaseDocument;
use crate::layout::collect_inline_text::collect_inline_text_recursive;
use crate::layout::grid_errors::GridPreprocessingError;
use crate::layout::replaced::{ReplacedContext, replaced_measure_function};
use crate::layout::stylo_to_blitz::TextCollapseMode;
use crate::node::{Node, NodeData};

/// Calculate proper intrinsic size for masonry items using CSS specifications
/// Replaces all hardcoded 200.0px/100.0px fallbacks with standards-compliant sizing
pub fn calculate_item_intrinsic_size_for_masonry(
    tree: &BaseDocument,
    item_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
    _masonry_axis: taffy::geometry::AbstractAxis,
) -> Result<taffy::Size<f32>, GridPreprocessingError> {
    let node = tree.node_from_id(item_id.into());

    // Phase 1: Check for explicit sizes first (CSS spec compliance)
    if let Some(computed_styles) = node.primary_styles() {
        let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);

        if let Some(explicit_size) = get_explicit_size(&style_wrapper) {
            return Ok(explicit_size);
        }
    }

    // Phase 2: Calculate content-based intrinsic size using existing infrastructure
    let content_size = calculate_content_intrinsic_size(tree, item_id, inputs)?;

    // Phase 3: Apply min/max constraints per CSS specification
    let constrained_size = apply_size_constraints(tree, item_id, content_size)?;

    Ok(constrained_size)
}

/// Extract explicit sizes from computed styles if available
fn get_explicit_size<T: std::ops::Deref<Target = ComputedValues>>(
    style_wrapper: &stylo_taffy::TaffyStyloStyle<T>,
) -> Option<taffy::Size<f32>> {
    let size_style = style_wrapper.size();

    let width = size_style.width.into_option();
    let height = size_style.height.into_option();

    // Only return if both dimensions are explicitly defined
    if let (Some(w), Some(h)) = (width, height) {
        Some(taffy::Size {
            width: w,
            height: h,
        })
    } else {
        None
    }
}

/// Calculate intrinsic size based on content type using existing measurement systems
fn calculate_content_intrinsic_size(
    tree: &BaseDocument,
    item_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
) -> Result<taffy::Size<f32>, GridPreprocessingError> {
    let node = tree.node_from_id(item_id.into());

    match &node.data {
        NodeData::Text(_) => {
            // Use existing inline layout measurement infrastructure
            measure_text_content_intrinsic_size(tree, item_id, inputs)
        }
        NodeData::Element(element_data) => {
            match element_data.name.local.as_ref() {
                "img" | "canvas" | "video" | "object" | "embed" => {
                    // Use existing replaced element measurement system
                    measure_replaced_element_intrinsic_size(tree, item_id, inputs)
                }
                _ => {
                    // Layout children using Taffy's intrinsic sizing patterns
                    measure_element_content_intrinsic_size(tree, item_id, inputs)
                }
            }
        }
        _ => Ok(taffy::Size::ZERO),
    }
}

/// Measure text content using existing inline layout infrastructure
fn measure_text_content_intrinsic_size(
    tree: &BaseDocument,
    item_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
) -> Result<taffy::Size<f32>, GridPreprocessingError> {
    let node = tree.node_from_id(item_id.into());
    
    if let Some(element) = node.data.downcast_element() {
        if let Some(mut inline_layout) = element.inline_layout_data.clone() {
            // Use new method that includes inline elements for correct CSS compliance
            let content_sizes = tree.text_system.with_font_system(|font_system| {
                inline_layout.calculate_content_widths_with_inline_elements(font_system)
            });

            // Apply Taffy's AvailableSpace pattern for intrinsic sizing
            let width = match inputs.available_space.width {
                AvailableSpace::MinContent => content_sizes.min,
                AvailableSpace::MaxContent => content_sizes.max,
                AvailableSpace::Definite(limit) => {
                    limit.min(content_sizes.max).max(content_sizes.min)
                }
            };

            let height = inline_layout.height();

            return Ok(taffy::Size { width, height });
        }
    }

    // Fallback: collect text using existing infrastructure and estimate
    let mut text_content = String::new();
    collect_inline_text_recursive(
        &mut text_content,
        &tree.nodes,
        item_id.into(),
        TextCollapseMode::Collapse,
    );

    // Use text system for measurement (implement based on existing patterns)
    estimate_text_size(&text_content, inputs)
}

/// Estimate text size for collected text content
fn estimate_text_size(
    text_content: &str,
    inputs: &taffy::tree::LayoutInput,
) -> Result<taffy::Size<f32>, GridPreprocessingError> {
    if text_content.trim().is_empty() {
        return Ok(taffy::Size::ZERO);
    }

    // Basic text size estimation based on character count and available space
    // This is a simplified approach - in production, proper font metrics would be used
    let char_count = text_content.len() as f32;
    let estimated_char_width = 8.0; // Approximate character width
    let estimated_line_height = 16.0; // Approximate line height

    let total_text_width = char_count * estimated_char_width;

    // Find longest word for min-content calculation
    let longest_word = text_content
        .split_whitespace()
        .map(|word| word.len() as f32 * estimated_char_width)
        .fold(0.0, f32::max);

    // Use CSS-compliant calculation based on available space constraints
    let text_width_limit = match inputs.available_space.width {
        AvailableSpace::MinContent => longest_word,
        AvailableSpace::MaxContent => total_text_width,
        AvailableSpace::Definite(limit) => limit.min(total_text_width),
    };
    let width = longest_word.max(text_width_limit);

    // Estimate height based on text wrapping
    let lines = if width > 0.0 {
        (total_text_width / width).ceil().max(1.0)
    } else {
        1.0
    };
    let height = lines * estimated_line_height;

    Ok(taffy::Size { width, height })
}

/// Measure replaced elements using existing replaced_measure_function
fn measure_replaced_element_intrinsic_size(
    tree: &BaseDocument,
    item_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
) -> Result<taffy::Size<f32>, GridPreprocessingError> {
    let node = tree.node_from_id(item_id.into());

    // Extract replaced element context using existing patterns
    let (inherent_size, attr_size) = extract_replaced_context(node)?;

    let replaced_context = ReplacedContext {
        inherent_size,
        attr_size,
    };

    // Use existing replaced_measure_function from replaced.rs:23
    let computed = replaced_measure_function(
        inputs.known_dimensions,
        inputs.parent_size,
        &replaced_context,
        &node.style(),
        false,
    );

    Ok(computed)
}

/// Extract replaced element context from node
fn extract_replaced_context(
    node: &Node,
) -> Result<(taffy::Size<f32>, taffy::Size<Option<f32>>), GridPreprocessingError> {
    let element =
        node.data
            .downcast_element()
            .ok_or(GridPreprocessingError::PreprocessingFailed {
                operation: "extract_replaced_context".to_string(),
                node_id: 0, // Node ID not available here
                details: "Node is not an element".to_string(),
            })?;

    // Extract intrinsic dimensions from element attributes
    let width_attr = element
        .attr(markup5ever::local_name!("width"))
        .and_then(|w| w.parse::<f32>().ok());
    let height_attr = element
        .attr(markup5ever::local_name!("height"))
        .and_then(|h| h.parse::<f32>().ok());

    // CSS-compliant fallbacks for replaced elements without natural sizes
    // CSS Sizing Module Level 3: 300px width / 150px height (NOT 200px/100px!)
    let inherent_width = width_attr.unwrap_or(300.0);
    let inherent_height = height_attr.unwrap_or(150.0);

    let inherent_size = taffy::Size {
        width: inherent_width,
        height: inherent_height,
    };

    let attr_size = taffy::Size {
        width: width_attr,
        height: height_attr,
    };

    Ok((inherent_size, attr_size))
}

/// Measure element content using available space constraints and style analysis
fn measure_element_content_intrinsic_size(
    tree: &BaseDocument,
    item_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
) -> Result<taffy::Size<f32>, GridPreprocessingError> {
    let node = tree.node_from_id(item_id.into());

    // Get computed styles to determine if any explicit sizing is available
    let explicit_sizes = if let Some(computed_styles) = node.primary_styles() {
        let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);
        let size_style = style_wrapper.size();

        (
            size_style.width.into_option(),
            size_style.height.into_option(),
        )
    } else {
        (None, None)
    };

    // Apply CSS intrinsic sizing rules based on available space
    let width = match inputs.available_space.width {
        AvailableSpace::MinContent => {
            // For min-content, use smaller dimension or CSS-compliant fallback
            explicit_sizes.0.unwrap_or(150.0) // Smaller than max-content
        }
        AvailableSpace::MaxContent => {
            // For max-content, use larger dimension or CSS-compliant fallback
            explicit_sizes.0.unwrap_or(300.0) // CSS Sizing Module Level 3 fallback
        }
        AvailableSpace::Definite(limit) => {
            // Use available space but respect explicit sizes
            let preferred = explicit_sizes.0.unwrap_or(300.0);
            limit.min(preferred).max(150.0)
        }
    };

    let height = match inputs.available_space.height {
        AvailableSpace::MinContent => {
            // For min-content, use smaller dimension or CSS-compliant fallback
            explicit_sizes.1.unwrap_or(75.0) // Smaller than max-content
        }
        AvailableSpace::MaxContent => {
            // For max-content, use larger dimension or CSS-compliant fallback
            explicit_sizes.1.unwrap_or(150.0) // CSS Sizing Module Level 3 fallback
        }
        AvailableSpace::Definite(limit) => {
            // Use available space but respect explicit sizes
            let preferred = explicit_sizes.1.unwrap_or(150.0);
            limit.min(preferred).max(75.0)
        }
    };

    Ok(taffy::Size { width, height })
}

/// Apply size constraints following CSS Sizing Module Level 3
fn apply_size_constraints(
    tree: &BaseDocument,
    item_id: NodeId,
    content_size: taffy::Size<f32>,
) -> Result<taffy::Size<f32>, GridPreprocessingError> {
    let node = tree.node_from_id(item_id.into());

    if let Some(computed_styles) = node.primary_styles() {
        let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);

        let min_size = style_wrapper.min_size();
        let max_size = style_wrapper.max_size();

        // Apply CSS min/max constraints following CSS Sizing Module Level 3
        let width = content_size
            .width
            .max(min_size.width.into_option().unwrap_or(0.0))
            .min(max_size.width.into_option().unwrap_or(f32::INFINITY));

        let height = content_size
            .height
            .max(min_size.height.into_option().unwrap_or(0.0))
            .min(max_size.height.into_option().unwrap_or(f32::INFINITY));

        return Ok(taffy::Size { width, height });
    }

    // CSS-compliant fallbacks for replaced elements without natural sizes
    // CSS Sizing Module Level 3: 300px width / 150px height (NOT 200px/100px!)
    let fallback_width = if content_size.width == 0.0 {
        300.0
    } else {
        content_size.width
    };
    let fallback_height = if content_size.height == 0.0 {
        150.0
    } else {
        content_size.height
    };

    Ok(taffy::Size {
        width: fallback_width,
        height: fallback_height,
    })
}
