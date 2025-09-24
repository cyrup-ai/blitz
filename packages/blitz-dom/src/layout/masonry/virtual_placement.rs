//! Virtual placement system for CSS Grid Level 3 masonry layout
//!
//! Implements virtual placement for spanning items as specified in the CSS Grid Level 3 specification.
//! Virtual placements ensure spanning items contribute to track sizing at all possible positions.

use taffy::geometry::AbstractAxis;
use taffy::prelude::NodeId;
use taffy::MaybeResolve;

use super::super::grid_errors::GridPreprocessingError;
use crate::BaseDocument;

/// Virtual placement representing a spanning item at a specific position
/// Implements CSS Grid Level 3 "every possible start position" requirement
#[derive(Debug, Clone)]
#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout
pub struct VirtualPlacement {
    pub item_id: NodeId,
    pub virtual_track_start: usize,
    pub track_span: usize,
    pub placement_weight: f32, // Distribution weight for intrinsic sizing
    pub intrinsic_contribution: f32, // Size contribution after gap subtraction
}

/// Grid item information for masonry placement
/// Enhanced with span information for proper masonry grid placement
#[derive(Debug, Clone)]
pub struct GridItemInfo {
    pub node_id: NodeId,
    pub order: usize,
    pub row_span: usize,
    pub column_span: usize,
}

/// Create virtual placements for all spanning items per CSS Grid Level 3
///
/// For each spanning item, creates virtual placements at ALL possible start positions
/// where the item could be placed, implementing the specification requirement:
/// "spanning items with automatic placement are assumed to be placed at every possible start position"
pub fn create_virtual_placements_for_spanning_items(
    tree: &BaseDocument,
    items: &[GridItemInfo],
    track_count: usize,
    masonry_axis: AbstractAxis,
    inputs: &taffy::tree::LayoutInput,
) -> Result<Vec<VirtualPlacement>, GridPreprocessingError> {
    let mut virtual_placements = Vec::new();

    for item in items {
        let span = match masonry_axis {
            AbstractAxis::Block => item.row_span,
            AbstractAxis::Inline => item.column_span,
        };

        // Only create virtual placements for spanning items (span > 1)
        if span > 1 {
            // Calculate item's intrinsic size contribution
            let item_size =
                super::item_collection::estimate_item_size_for_masonry(tree, item.node_id, inputs)?;
            let base_intrinsic_size = match masonry_axis {
                AbstractAxis::Block => item_size.height,
                AbstractAxis::Inline => item_size.width,
            };

            // CSS Grid Level 3: "subtract the combined size of the gaps it would span"
            let gap_size = calculate_gap_size_for_span(tree, span, masonry_axis)?;
            let adjusted_intrinsic_size = (base_intrinsic_size - gap_size).max(0.0);

            // CSS Grid Level 3: "divide by its span"
            let per_track_contribution = adjusted_intrinsic_size / span as f32;

            // Create virtual placement at EVERY possible start position
            for start_track in 0..=(track_count.saturating_sub(span)) {
                virtual_placements.push(VirtualPlacement {
                    item_id: item.node_id,
                    virtual_track_start: start_track,
                    track_span: span,
                    placement_weight: 1.0 / (track_count - span + 1) as f32,
                    intrinsic_contribution: per_track_contribution,
                });
            }
        }
    }

    Ok(virtual_placements)
}

/// Calculate gap size that a spanning item would span
/// Per CSS Grid Level 3: "subtract the combined size of the gaps it would span"
fn calculate_gap_size_for_span(
    tree: &BaseDocument,
    span: usize,
    masonry_axis: AbstractAxis,
) -> Result<f32, GridPreprocessingError> {
    if span <= 1 {
        return Ok(0.0);
    }

    // Extract actual gap size from computed styles using Taffy's CSS property resolution
    let gap_per_track = extract_grid_gap_from_styles(tree, masonry_axis)?;
    let total_gaps = (span - 1) as f32;

    Ok(gap_per_track * total_gaps)
}

/// Extract grid gap from computed styles
/// Uses CSS Grid gap properties to get actual gap values instead of hardcoded defaults
fn extract_grid_gap_from_styles(
    tree: &BaseDocument,
    masonry_axis: AbstractAxis,
) -> Result<f32, GridPreprocessingError> {
    // Need container context to extract gap - use root container as representative
    let root_node = tree.root_node();
    
    if let Some(computed_styles) = root_node.primary_styles() {
        // Extract gap directly from computed styles
        let position_styles = computed_styles.get_position();
        let gap = match masonry_axis {
            AbstractAxis::Block => stylo_taffy::convert::gap(&position_styles.row_gap),
            AbstractAxis::Inline => stylo_taffy::convert::gap(&position_styles.column_gap),
        };
        
        Ok(gap.maybe_resolve(0.0, crate::layout::resolve_calc_value).unwrap_or(0.0))
    } else {
        // Fallback to CSS Grid spec default for gap: normal (0px)
        Ok(0.0)
    }
}

/// Enhanced track sizing that properly handles spanning items with virtual placement
/// Replaces calculate_track_intrinsic_size to implement CSS Grid Level 3 specification
#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout
pub fn calculate_track_intrinsic_size_with_spanning(
    tree: &BaseDocument,
    regular_items: &[GridItemInfo],
    virtual_placements: &[VirtualPlacement],
    track_idx: usize,
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<f32, GridPreprocessingError> {
    let mut max_intrinsic_size: f32 = 0.0;

    // Step 1: Process regular non-spanning items (span = 1)
    for item in regular_items {
        let span = match masonry_axis {
            AbstractAxis::Block => item.row_span,
            AbstractAxis::Inline => item.column_span,
        };

        if span == 1 {
            let item_size =
                super::item_collection::estimate_item_size_for_masonry(tree, item.node_id, inputs)?;
            let contribution = match masonry_axis {
                AbstractAxis::Block => item_size.height,
                AbstractAxis::Inline => item_size.width,
            };
            max_intrinsic_size = max_intrinsic_size.max(contribution);
        }
    }

    // Step 2: Process virtual spanning item placements
    // CSS Grid Level 3: spanning items contribute to ALL tracks they could overlap
    for virtual_placement in virtual_placements {
        let start = virtual_placement.virtual_track_start;
        let end = start + virtual_placement.track_span;

        // Check if this virtual placement overlaps with current track
        if track_idx >= start && track_idx < end {
            max_intrinsic_size = max_intrinsic_size.max(virtual_placement.intrinsic_contribution);
        }
    }

    // Let CSS constraints handle minimums properly - no hardcoded values
    Ok(max_intrinsic_size.max(0.0))
}
