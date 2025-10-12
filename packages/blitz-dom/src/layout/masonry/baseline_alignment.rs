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
use crate::node::ElementData;
use crate::BaseDocument;

// Import traits to use align_self() and justify_self() methods
use taffy::FlexboxItemStyle;
use taffy::GridItemStyle;
use taffy::ResolveOrZero;

/// Baseline alignment information for a masonry item
#[derive(Debug, Clone)]
pub struct MasonryItemBaseline {
    /// The item's node ID
    #[allow(dead_code)] // False positive: field IS used extensively, compiler confused by derived traits
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
    /// Track actual masonry-axis position for this group
    pub position: f32,
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
        // Use explicit trait methods to disambiguate
        let align_value = match masonry_axis {
            AbstractAxis::Block => FlexboxItemStyle::align_self(&style_wrapper),
            AbstractAxis::Inline => GridItemStyle::justify_self(&style_wrapper),
        };

        // Check for baseline alignment values
        // Note: Taffy currently only supports Baseline (not FirstBaseline/LastBaseline variants)
        matches!(align_value, Some(taffy::AlignSelf::Baseline))
    } else {
        false
    }
}

/// Extract baseline from layout output (NEW - Taffy's approach)
/// This is called AFTER the item has been laid out
pub fn extract_item_baseline_from_layout(
    tree: &BaseDocument,
    item_id: NodeId,
    layout_output: &taffy::tree::LayoutOutput,
    masonry_axis: AbstractAxis,
    container_size: taffy::Size<Option<f32>>,
) -> Option<f32> {
    // Method 1: Extract from layout output (PRIMARY - from Taffy's grid)
    // Pattern from: /tmp/taffy/src/compute/grid/track_sizing.rs:501-507
    if let Some(baseline_y) = layout_output.first_baselines.y {
        // Baseline includes top margin per CSS spec
        let top_margin = extract_top_margin(tree, item_id, masonry_axis, container_size);
        
        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Item {} baseline from layout: {} + margin: {} = {}",
            usize::from(item_id),
            baseline_y,
            top_margin,
            baseline_y + top_margin
        );
        
        return Some(baseline_y + top_margin);
    }

    // Method 2: Fallback to font metrics for simple text
    let node = tree.node_from_id(item_id.into());
    if let Some(element) = node.data.downcast_element() {
        if let Some(styles) = node.primary_styles() {
            if let Some(baseline) = calculate_baseline_from_font_metrics(tree, element, &*styles) {
                let top_margin = extract_top_margin(tree, item_id, masonry_axis, container_size);
                return Some(baseline + top_margin);
            }
        }
    }

    // Method 3: No baseline - will use height as fallback
    None
}

/// Extract baseline offset for a masonry item (DEPRECATED - use extract_item_baseline_from_layout)
#[allow(dead_code)]
pub fn extract_item_baseline(
    tree: &BaseDocument,
    item_id: NodeId,
) -> Option<f32> {
    let node = tree.node_from_id(item_id.into());

    // Method 1: Calculate from font metrics for text content
    if let Some(element) = node.data.downcast_element() {
        if let Some(styles) = node.primary_styles() {
            return calculate_baseline_from_font_metrics(tree, element, &*styles);
        }
    }

    // Method 2: For replaced elements, use margin box bottom
    None // Item has no baseline, will use height as fallback
}

/// Calculate baseline from font metrics
fn calculate_baseline_from_font_metrics(
    tree: &BaseDocument,
    _element: &ElementData,
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
    container_size: taffy::Size<Option<f32>>,
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

        // ✅ FIX: Use container size from inputs, not final_layout
        // Per CSS spec, percentage margins resolve against the INLINE size
        let parent_inline_size = container_size.width;
        
        // Resolve margin to pixels (Auto resolves to 0.0)
        taffy_margin.resolve_or_zero(parent_inline_size, |_, _| 0.0)
    } else {
        0.0
    }
}

/// Calculate baseline adjustments for all placed items
pub fn calculate_baseline_adjustments(
    tree: &BaseDocument,
    placed_items: &[(NodeId, stylo_taffy::GridArea)],
    layout_outputs: &[taffy::tree::LayoutOutput],  // ✅ NEW: Layout outputs for baseline extraction
    masonry_axis: AbstractAxis,
    container_size: taffy::Size<Option<f32>>,
) -> Result<Vec<BaselineAdjustment>, GridPreprocessingError> {
    // Step 1: Extract baseline info for all items that need baseline alignment
    let mut baseline_items: Vec<(usize, MasonryItemBaseline)> = Vec::new();

    for (idx, (item_id, grid_area)) in placed_items.iter().enumerate() {
        if !should_align_baseline(tree, *item_id, masonry_axis) {
            continue; // Skip items not using baseline alignment
        }

        // ✅ Extract baseline from layout output (Taffy's approach)
        let layout_output = &layout_outputs[idx];
        let baseline_offset = extract_item_baseline_from_layout(
            tree, 
            *item_id, 
            layout_output,
            masonry_axis,
            container_size,
        );
        
        let top_margin = extract_top_margin(tree, *item_id, masonry_axis, container_size);
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
                max_baseline: 0.0,
                position: 0.0,
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
                        // ✅ FIX: baseline_offset already includes top_margin from extract_item_baseline_from_layout()
                        // Don't add it again! (Taffy pattern: margin added once when storing baseline)
                        item.baseline_offset.unwrap_or(item.item_height + item.top_margin)
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
                // ✅ FIX: baseline_offset already includes top_margin, don't add it twice!
                let item_baseline = item.baseline_offset.unwrap_or(item.item_height + item.top_margin);

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
