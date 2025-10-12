//! Layout output generation for CSS Grid Level 3 masonry layout
//!
//! Handles the final phase of masonry layout by generating layout outputs for placed items.

use taffy::prelude::NodeId;
use taffy::{Layout, LayoutPartialTree, Point, Rect, Size, geometry::AbstractAxis};

use super::super::grid_errors::GridPreprocessingError;
use super::taffy_integration::{calculate_container_size_from_placements, grid_area_to_layout};
use crate::BaseDocument;

/// Layout masonry items and return their layout outputs for baseline calculation
/// This must be called BEFORE baseline adjustments are calculated
pub fn layout_masonry_items(
    tree: &mut BaseDocument,
    placed_items: &[(NodeId, stylo_taffy::GridArea)],
    inputs: taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
    track_sizes: &[f32],
    gap_size: f32,
) -> Result<Vec<taffy::tree::LayoutOutput>, GridPreprocessingError> {
    let mut layout_outputs = Vec::with_capacity(placed_items.len());

    // Layout each item and its children (following Taffy's grid pattern)
    // Pattern from: /tmp/taffy/src/compute/grid/alignment.rs:202-253
    for (item_id, grid_area) in placed_items {
        let (_, size) = grid_area_to_layout(grid_area, masonry_axis, track_sizes, gap_size);

        // Layout the item (this recursively lays out all children)
        let layout_output = tree.compute_child_layout(
            *item_id,
            taffy::tree::LayoutInput {
                known_dimensions: Size { 
                    width: Some(size.width), 
                    height: Some(size.height) 
                },
                parent_size: inputs.known_dimensions,
                available_space: Size {
                    width: taffy::AvailableSpace::Definite(size.width),
                    height: taffy::AvailableSpace::Definite(size.height),
                },
                sizing_mode: taffy::SizingMode::InherentSize,
                axis: taffy::RequestedAxis::Both,
                run_mode: taffy::RunMode::PerformLayout,
                vertical_margins_are_collapsible: taffy::Line::FALSE,
            },
        );

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Laid out masonry item {} with size=({}, {}), baseline={:?}",
            usize::from(*item_id),
            size.width,
            size.height,
            layout_output.first_baselines.y
        );

        layout_outputs.push(layout_output);
    }

    Ok(layout_outputs)
}

/// Apply final positions to laid-out masonry items
/// This must be called AFTER baseline adjustments have been applied to grid_areas
pub fn apply_masonry_positions(
    tree: &mut BaseDocument,
    placed_items: &[(NodeId, stylo_taffy::GridArea)],
    layout_outputs: &[taffy::tree::LayoutOutput],
    masonry_axis: AbstractAxis,
    track_sizes: &[f32],
    gap_size: f32,
) {
    // Apply final positions to each item
    for (idx, (item_id, grid_area)) in placed_items.iter().enumerate() {
        let (location, size) = grid_area_to_layout(grid_area, masonry_axis, track_sizes, gap_size);
        let layout_output = &layout_outputs[idx];

        let item_layout = Layout {
            order: 0,
            location,
            size,
            content_size: layout_output.content_size,
            scrollbar_size: Size::ZERO,
            border: Rect::ZERO,
            padding: Rect::ZERO,
            margin: Rect::ZERO,
        };

        tree.set_unrounded_layout(*item_id, &item_layout);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Set masonry item {} final position: ({}, {})",
            usize::from(*item_id),
            location.x,
            location.y
        );
    }
}

/// Generate final container layout output
pub fn generate_container_output(
    placed_items: &[(NodeId, stylo_taffy::GridArea)],
    masonry_axis: AbstractAxis,
    inputs: taffy::tree::LayoutInput,
    track_sizes: &[f32],
    gap_size: f32,
) -> taffy::tree::LayoutOutput {
    let container_size = calculate_container_size_from_placements(
        placed_items,
        masonry_axis,
        inputs.available_space,
        track_sizes,
        gap_size,
    );

    taffy::LayoutOutput {
        size: container_size,
        content_size: container_size,
        first_baselines: Point::NONE,
        top_margin: taffy::CollapsibleMarginSet::ZERO,
        bottom_margin: taffy::CollapsibleMarginSet::ZERO,
        margins_can_collapse_through: false,
    }
}
