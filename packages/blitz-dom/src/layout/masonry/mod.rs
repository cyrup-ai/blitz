//! CSS Grid Level 3 masonry layout implementation
//!
//! This module implements the "shortest track placement" algorithm
//! for masonry layout as specified in the CSS Grid Level 3 specification.

use taffy::GridContainerStyle;
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

    // Phase 5: Place items using pre-sized tracks with optional dense packing
    let mut placed_items = Vec::new();

    for item in grid_items {
        let item_span = match masonry_axis {
            AbstractAxis::Block => item.row_span,
            AbstractAxis::Inline => item.column_span,
        };

        // Determine placement track: try gap first if dense packing enabled
        let placement_track = if config.dense_packing {
            // Calculate item size for gap fitting
            let item_size = item_collection::estimate_item_size_for_masonry(
                tree,
                item.node_id,
                &inputs,
            )?;
            let item_masonry_size = match masonry_axis {
                AbstractAxis::Block => item_size.height,
                AbstractAxis::Inline => item_size.width,
            };

            // Calculate normal placement track size (for compatibility check)
            let shortest_track = masonry_state.find_shortest_track_with_tolerance();
            let normal_track_size: f32 = (shortest_track..(shortest_track + item_span))
                .map(|i| track_sizes.get(i).copied().unwrap_or(0.0))
                .sum();

            // Detect compatible gaps
            let gaps = gap_detection::detect_compatible_gaps(
                &masonry_state,
                &track_sizes,
                item_span,
                item_masonry_size,
                normal_track_size,
                config.item_tolerance,
            );

            // Use first (earliest) gap if available, otherwise use shortest track
            gaps.first()
                .map(|gap| gap.track_index)
                .unwrap_or(shortest_track)
        } else {
            // Standard shortest track placement (no dense packing)
            masonry_state.find_shortest_track_with_tolerance()
        };

        // Place item using determined track
        let placement = item_collection::place_item_in_taffy_sized_track(
            tree,
            &item,
            placement_track,
            &track_sizes[placement_track], // Use actual Taffy track size
            &masonry_state,
            masonry_axis,
            &inputs,
        )?;

        // Record placement in masonry state
        let item_size_for_track = placement.1.masonry_axis_size;

        masonry_state.place_item_with_tracking(placement_track, item_size_for_track, item_span);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Placed masonry item {} at track {} position {}",
            usize::from(item.node_id),
            placement_track,
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

    // Phase 5.7: Layout items first to extract baselines
    let layout_outputs = layout_output::layout_masonry_items(
        tree,
        &placed_items,
        inputs,
        masonry_axis,
        &track_sizes,
    )?;

    // Phase 5.8: Calculate baseline alignment adjustments using layout outputs
    let container_size = inputs.known_dimensions;
    let baseline_adjustments = baseline_alignment::calculate_baseline_adjustments(
        tree,
        &placed_items,
        &layout_outputs,  // ✅ Pass layout outputs for baseline extraction
        masonry_axis,
        container_size,
    )?;

    // Phase 5.9: Apply baseline adjustments to masonry_axis_position
    for adjustment in baseline_adjustments {
        if let Some((_, grid_area)) = placed_items.get_mut(adjustment.item_index) {
            grid_area.masonry_axis_position += adjustment.position_adjustment;
        }
    }

    // Phase 6: Apply final positions with adjustments
    layout_output::apply_masonry_positions(
        tree,
        &placed_items,
        &layout_outputs,
        masonry_axis,
        &track_sizes,
    );

    // Phase 7: Generate container layout output
    Ok(layout_output::generate_container_output(
        &placed_items,
        masonry_axis,
        inputs,
        &track_sizes,
    ))
}

/// Check if tracks contain auto-fit repeat
fn has_auto_fit_tracks<'a, I>(mut tracks: I) -> bool
where
    I: Iterator<Item = taffy::GenericGridTemplateComponent<String, &'a taffy::GridTemplateRepetition<String>>>,
{
    tracks.any(|component| match component {
        taffy::GenericGridTemplateComponent::Single(_) => false,
        taffy::GenericGridTemplateComponent::Repeat(repeat) => {
            matches!(repeat.count, taffy::RepetitionCount::AutoFit)
        }
    })
}

/// Collapse empty auto-fit tracks after placement
fn collapse_empty_auto_fit_tracks(
    placed_items: &mut Vec<(NodeId, stylo_taffy::GridArea)>,
    track_sizes: &[f32],
    _masonry_axis: AbstractAxis,
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
