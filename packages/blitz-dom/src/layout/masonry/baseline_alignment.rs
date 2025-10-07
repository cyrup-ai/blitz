//! Baseline alignment for CSS Grid Level 3 masonry layout
//!
//! Implements baseline alignment for masonry items, allowing items at the same
//! grid-axis position to align their baselines, following the pattern from Taffy's
//! grid baseline alignment implementation.

use std::collections::HashMap;
use taffy::geometry::AbstractAxis;
use taffy::prelude::NodeId;

use super::super::grid_errors::GridPreprocessingError;
use super::super::intrinsic_sizing::extract_font_metrics_fallback;
use crate::node::ElementNodeData;
use crate::BaseDocument;

/// Baseline alignment information for a masonry item
#[derive(Debug, Clone)]
pub struct MasonryItemBaseline {
    /// The item's node ID
    pub node_id: NodeId,
    /// Grid-axis track position (for grouping)
    pub grid_axis_track: usize,
    /// Baseline offset from item's content box top (in pixels)
    pub baseline_offset: Option<f32>,
    /// Item's height (used as fallback if no baseline)
    pub item_height: f32,
    /// Top margin (included in baseline calculation)
    pub top_margin: f32,
}

/// A group of masonry items that share baseline alignment
#[derive(Debug)]
pub struct BaselineGroup {
    /// All items in this group (same grid-axis position)
    pub items: Vec<usize>, // Indices into baseline_items array
    /// Maximum baseline in this group
    pub max_baseline: f32,
}

/// Result of baseline alignment calculation
#[derive(Debug)]
pub struct BaselineAdjustment {
    /// Item index in placed_items
    pub item_index: usize,
    /// Offset to add to masonry_axis_position (the "shim")
    pub position_adjustment: f32,
}

/// Check if an item should participate in baseline alignment
pub fn should_align_baseline(
    tree: &BaseDocument,
    item_id: NodeId,
    masonry_axis: AbstractAxis,
) -> bool {
    let node = tree.node_from_id(item_id.into());

    if let Some(styles) = node.primary_styles() {
        let style_wrapper = stylo_taffy::TaffyStyloStyle::from(styles);

        // Check align-self in the masonry axis direction
        let align_value = match masonry_axis {
            AbstractAxis::Block => style_wrapper.align_self(),
            AbstractAxis::Inline => style_wrapper.justify_self(),
        };

        // Check for baseline alignment values
        // Note: Taffy currently only supports Baseline (not FirstBaseline/LastBaseline variants)
        matches!(align_value, Some(taffy::AlignItems::Baseline))
    } else {
        false
    }
}

/// Extract baseline offset for a masonry item
pub fn extract_item_baseline(
    tree: &BaseDocument,
    item_id: NodeId,
) -> Option<f32> {
    let node = tree.node_from_id(item_id.into());

    // Method 1: Check if item has LayoutOutput with baseline
    // (Currently all return Point::NONE, but this prepares for future)
    let layout = node.unrounded_layout;
    if layout.first_baselines.y.is_some() {
        return layout.first_baselines.y;
    }

    // Method 2: Calculate from font metrics for text content
    if let Some(element) = node.data.downcast_element() {
        if let Some(styles) = node.primary_styles() {
            return calculate_baseline_from_font_metrics(tree, element, styles);
        }
    }

    // Method 3: For replaced elements, use margin box bottom
    // (per CSS spec: replaced elements use their margin box)
    None // Item has no baseline, will use height as fallback
}

/// Calculate baseline from font metrics
fn calculate_baseline_from_font_metrics(
    tree: &BaseDocument,
    _element: &ElementNodeData,
    styles: &style::properties::ComputedValues,
) -> Option<f32> {
    let font = styles.get_font();
    let font_size = font.font_size.computed_size.px();

    // Extract font family name (reuse pattern from intrinsic_sizing.rs:164-183)
    let font_family: &str = match font.font_family.families.iter().next() {
        Some(family) => {
            use style::values::computed::font::SingleFontFamily;
            match family {
                SingleFontFamily::FamilyName(name) => name.name.as_ref(),
                SingleFontFamily::Generic(generic) => {
                    use style::values::computed::font::GenericFontFamily;
                    match generic {
                        GenericFontFamily::Serif => "serif",
                        GenericFontFamily::SansSerif => "sans-serif",
                        GenericFontFamily::Monospace => "monospace",
                        GenericFontFamily::Cursive => "cursive",
                        GenericFontFamily::Fantasy => "fantasy",
                        _ => "sans-serif",
                    }
                }
            }
        }
        None => "sans-serif",
    };

    // Get font metrics (reuse existing infrastructure)
    let metrics = extract_font_metrics_fallback(tree, font_family, font_size);

    // Baseline = ascent scaled to font size
    // (This is the alphabetic baseline, standard for Latin text)
    let scale = font_size / metrics.units_per_em as f32;
    let baseline_offset = metrics.ascent as f32 * scale;

    Some(baseline_offset)
}

/// Extract top margin for an item
fn extract_top_margin(
    tree: &BaseDocument,
    item_id: NodeId,
    masonry_axis: AbstractAxis,
) -> f32 {
    let node = tree.node_from_id(item_id.into());

    if let Some(styles) = node.primary_styles() {
        let margin = styles.get_margin();

        // For masonry, the "top" margin depends on the masonry axis direction
        let margin_value = match masonry_axis {
            AbstractAxis::Block => &margin.margin_top,  // Flowing vertically, use top margin
            AbstractAxis::Inline => &margin.margin_left, // Flowing horizontally, use left margin
        };

        // Convert stylo margin to taffy and resolve to pixels
        use stylo_taffy::convert;
        let taffy_margin = convert::margin(margin_value);

        // Get parent size for percentage resolution (if available)
        let parent_size = node.final_layout.size;
        let parent_dimension = match masonry_axis {
            AbstractAxis::Block => parent_size.height,
            AbstractAxis::Inline => parent_size.width,
        };

        // Resolve margin to pixels (Auto resolves to 0.0)
        taffy_margin.resolve_or_zero(Some(parent_dimension))
    } else {
        0.0
    }
}

/// Calculate baseline adjustments for all placed items
pub fn calculate_baseline_adjustments(
    tree: &BaseDocument,
    placed_items: &[(NodeId, stylo_taffy::GridArea)],
    masonry_axis: AbstractAxis,
) -> Result<Vec<BaselineAdjustment>, GridPreprocessingError> {
    // Step 1: Extract baseline info for all items that need baseline alignment
    let mut baseline_items: Vec<(usize, MasonryItemBaseline)> = Vec::new();

    for (idx, (item_id, grid_area)) in placed_items.iter().enumerate() {
        if !should_align_baseline(tree, *item_id, masonry_axis) {
            continue; // Skip items not using baseline alignment
        }

        let baseline_offset = extract_item_baseline(tree, *item_id);
        let top_margin = extract_top_margin(tree, *item_id, masonry_axis);
        let item_height = grid_area.masonry_axis_size;
        let grid_axis_track = grid_area.grid_axis_start;

        baseline_items.push((idx, MasonryItemBaseline {
            node_id: *item_id,
            grid_axis_track,
            baseline_offset,
            item_height,
            top_margin,
        }));
    }

    // If no items need baseline alignment, return empty adjustments
    if baseline_items.is_empty() {
        return Ok(Vec::new());
    }

    // Step 2: Group items by grid-axis track
    let mut groups: HashMap<usize, BaselineGroup> = HashMap::new();
    for (placed_idx, item) in &baseline_items {
        groups.entry(item.grid_axis_track)
            .or_insert_with(|| BaselineGroup {
                items: Vec::new(),
                max_baseline: 0.0
            })
            .items.push(*placed_idx);
    }

    // Step 3: Calculate max baseline per group
    for group in groups.values_mut() {
        let max = group.items.iter()
            .filter_map(|&placed_idx| {
                // Find the baseline item that corresponds to this placed_idx
                baseline_items.iter()
                    .find(|(idx, _)| *idx == placed_idx)
                    .map(|(_, item)| {
                        // Baseline includes top margin, same as Taffy pattern
                        item.baseline_offset.unwrap_or(item.item_height) + item.top_margin
                    })
            })
            .fold(f32::NEG_INFINITY, f32::max);

        // Only set max_baseline if we found valid values
        if max.is_finite() && max > 0.0 {
            group.max_baseline = max;
        }
    }

    // Step 4: Calculate adjustments (the "shim")
    let mut adjustments = Vec::new();
    for group in groups.values() {
        // Skip groups with no valid baseline
        if group.max_baseline <= 0.0 {
            continue;
        }

        for &placed_idx in &group.items {
            // Find the baseline item for this placed index
            if let Some((_, item)) = baseline_items.iter().find(|(idx, _)| *idx == placed_idx) {
                let item_baseline = item.baseline_offset.unwrap_or(item.item_height) + item.top_margin;

                // Shim = max_baseline - item_baseline
                // (Same formula as Taffy: track_sizing.rs:516)
                let shim = group.max_baseline - item_baseline;

                if shim > 0.0 {
                    adjustments.push(BaselineAdjustment {
                        item_index: placed_idx,
                        position_adjustment: shim,
                    });
                }
            }
        }
    }

    Ok(adjustments)
}
