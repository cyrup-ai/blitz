//! Layout output generation for CSS Grid Level 3 masonry layout
//!
//! Handles the final phase of masonry layout by generating layout outputs for placed items.

use taffy::prelude::NodeId;
use taffy::{Layout, LayoutPartialTree, Point, Rect, Size, geometry::AbstractAxis};

use super::super::grid_errors::GridPreprocessingError;
use super::taffy_integration::{calculate_container_size_from_placements, grid_area_to_layout};
use crate::BaseDocument;

/// Generate final layout output with masonry placements
pub fn generate_masonry_layout_output(
    tree: &mut BaseDocument,
    _container_id: NodeId,
    inputs: taffy::tree::LayoutInput,
    placed_items: Vec<(NodeId, stylo_taffy::GridArea)>,
    masonry_axis: AbstractAxis,
    track_sizes: &[f32],
) -> Result<taffy::tree::LayoutOutput, GridPreprocessingError> {
    // Phase 1: Calculate container size based on actual item placements
    let container_size = calculate_container_size_from_placements(
        &placed_items,
        masonry_axis,
        inputs.available_space,
        track_sizes,
    );

    // Phase 2: Set final layout for each placed item
    for (item_id, grid_area) in &placed_items {
        let (location, size) = grid_area_to_layout(grid_area, masonry_axis, track_sizes);

        // Create proper Layout structure following existing patterns
        let item_layout = Layout {
            order: 0,
            size,
            content_size: size,
            location,
            scrollbar_size: Size::ZERO,
            border: Rect::ZERO,
            padding: Rect::ZERO,
            margin: Rect::ZERO,
        };

        // Set the layout using existing infrastructure
        tree.set_unrounded_layout(*item_id, &item_layout);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Set masonry item {} layout: pos=({}, {}), size=({}, {})",
            usize::from(*item_id),
            location.x,
            location.y,
            size.width,
            size.height
        );
    }

    // Phase 3: Return proper container layout output
    Ok(taffy::LayoutOutput {
        size: container_size,
        content_size: container_size,
        first_baselines: Point::NONE,
        top_margin: taffy::CollapsibleMarginSet::ZERO,
        bottom_margin: taffy::CollapsibleMarginSet::ZERO,
        margins_can_collapse_through: false,
    })
}
