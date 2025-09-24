//! CSS Grid Level 3 masonry layout implementation
//!
//! This module implements the "shortest track placement" algorithm
//! for masonry layout as specified in the CSS Grid Level 3 specification.

use taffy::geometry::AbstractAxis;
use taffy::prelude::NodeId;

use super::grid_errors::GridPreprocessingError;
use crate::BaseDocument;

// Internal modules only - no public re-exports needed as functions are used with full paths

// Internal modules
mod item_collection;
mod layout_output;
mod taffy_integration;
mod track_counting;
mod track_sizing;
mod virtual_placement;

/// Apply CSS Grid Level 3 masonry layout algorithm
/// Implements the two-phase algorithm: track sizing before item placement per CSS spec
pub fn apply_masonry_layout(
    tree: &mut BaseDocument,
    node_id: NodeId,
    inputs: taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<taffy::tree::LayoutOutput, GridPreprocessingError> {
    use stylo_taffy::MasonryTrackState;

    // Phase 1: Size tracks before any item placement ✨ NEW
    let track_sizes =
        track_sizing::size_masonry_tracks_before_placement(tree, node_id, &inputs, masonry_axis)?;

    // Phase 2: Get masonry configuration using existing infrastructure ✨ WARNING 12
    let config = item_collection::calculate_masonry_config(tree, node_id)?;

    // Phase 3: Initialize masonry state with configuration ✨ WARNING 11
    let mut masonry_state =
        MasonryTrackState::new_with_tolerance(config.track_count, config.item_tolerance);

    // Phase 4: Collect and sort items by placement order ✨ WARNING 10
    let grid_items = item_collection::collect_and_sort_masonry_items(tree, node_id)?;

    // Phase 5: Place items using pre-sized tracks
    let mut placed_items = Vec::new();

    for item in grid_items {
        // Find shortest track using tolerance-based algorithm
        let shortest_track = masonry_state.find_shortest_track_with_tolerance();

        // Place item using Taffy-sized track information
        let placement = item_collection::place_item_in_taffy_sized_track(
            tree,
            &item,
            shortest_track,
            &track_sizes[shortest_track], // Use actual Taffy track size
            &masonry_state,
            masonry_axis,
            &inputs,
        )?;

        // Record placement in masonry state
        let item_size_for_track = match masonry_axis {
            AbstractAxis::Block => placement.1.masonry_axis_size,
            AbstractAxis::Inline => placement.1.masonry_axis_size,
        };

        let span = match masonry_axis {
            AbstractAxis::Block => item.row_span, // ✨ WARNING 10 field usage
            AbstractAxis::Inline => item.column_span, // ✨ WARNING 10 field usage
        };

        masonry_state.place_item_with_tracking(shortest_track, item_size_for_track, span);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Placed masonry item {} at track {} position {}",
            usize::from(item.node_id),
            shortest_track,
            placement.1.masonry_axis_position
        );

        placed_items.push(placement);
    }

    // Phase 6: Generate layout output with masonry placements
    layout_output::generate_masonry_layout_output(
        tree,
        node_id,
        inputs,
        placed_items,
        masonry_axis,
        &track_sizes,
    )
}
