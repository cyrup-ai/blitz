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
mod baseline_alignment;
mod gap_detection;
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

    // Phase 2: Get masonry configuration with auto-repeat support ✨ WARNING 12
    let config = item_collection::calculate_masonry_config(tree, node_id, &inputs)?;

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

    // Phase 5.5: Collapse empty tracks for auto-fit (optional enhancement)
    let node = tree.node_from_id(node_id.into());
    if let Some(styles) = node.primary_styles() {
        let style_wrapper = stylo_taffy::TaffyStyloStyle::from(styles);
        let tracks = match masonry_axis {
            AbstractAxis::Block => style_wrapper.grid_template_columns(),
            AbstractAxis::Inline => style_wrapper.grid_template_rows(),
        };

        if let Some(tracks) = tracks {
            if has_auto_fit_tracks(tracks) {
                collapse_empty_auto_fit_tracks(&mut placed_items, &track_sizes, masonry_axis);
            }
        }
    }

    // Phase 5.7: Apply baseline alignment adjustments
    let baseline_adjustments = baseline_alignment::calculate_baseline_adjustments(
        tree,
        &placed_items,
        masonry_axis,
    )?;

    // Apply baseline adjustments to masonry_axis_position
    for adjustment in baseline_adjustments {
        if let Some((_, grid_area)) = placed_items.get_mut(adjustment.item_index) {
            grid_area.masonry_axis_position += adjustment.position_adjustment;
        }
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

/// Check if tracks contain auto-fit repeat
fn has_auto_fit_tracks<'a, I>(tracks: I) -> bool
where
    I: Iterator<Item = taffy::GenericGridTemplateComponent<String, &'a taffy::GridTemplateRepetition<String>>>,
{
    tracks.any(|component| match component {
        taffy::GenericGridTemplateComponent::Single(_) => false,
        taffy::GenericGridTemplateComponent::Repeat(repeat) => {
            matches!(repeat.count(), taffy::RepetitionCount::AutoFit)
        }
    })
}

/// Collapse empty auto-fit tracks after placement
fn collapse_empty_auto_fit_tracks(
    placed_items: &mut Vec<(NodeId, stylo_taffy::GridArea)>,
    track_sizes: &[f32],
    masonry_axis: AbstractAxis,
) {
    use std::collections::HashSet;

    // Determine which tracks have items
    let mut tracks_with_items = HashSet::new();
    for (_node_id, grid_area) in placed_items.iter() {
        tracks_with_items.insert(grid_area.grid_axis_start);
    }

    // Calculate cumulative offset from collapsed tracks
    let mut cumulative_offset = 0.0;
    let mut track_offsets = vec![0.0; track_sizes.len()];

    for (idx, &size) in track_sizes.iter().enumerate() {
        track_offsets[idx] = cumulative_offset;
        if !tracks_with_items.contains(&idx) {
            // Track is empty - will be collapsed
            cumulative_offset += size;
        }
    }

    // Apply offsets to shift items in later tracks
    for (_node_id, grid_area) in placed_items.iter_mut() {
        let track_idx = grid_area.grid_axis_start;
        if track_idx < track_offsets.len() {
            grid_area.masonry_axis_position -= track_offsets[track_idx];
        }
    }
}
