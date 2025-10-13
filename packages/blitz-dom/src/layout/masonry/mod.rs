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
/// 
/// Masonry axis is determined automatically from styles - no need to pass it in.
/// Config extracts both masonry_axis (flow direction) and grid_axis (track direction).
pub fn apply_masonry_layout(
    tree: &mut BaseDocument,
    node_id: NodeId,
    inputs: taffy::tree::LayoutInput,
) -> Result<taffy::tree::LayoutOutput, GridPreprocessingError> {
    use stylo_taffy::MasonryTrackState;

    // Phase 1: Get masonry configuration with auto-repeat support (MUST BE FIRST)
    // Config now includes both masonry_axis and grid_axis
    let config = item_collection::calculate_masonry_config(tree, node_id, &inputs)?;

    // Phase 2: Size tracks using the track count from config
    let track_sizes = track_sizing::size_masonry_tracks_before_placement(
        tree, 
        node_id, 
        &inputs, 
        config.masonry_axis,  // Use axis from config
        config.track_count
    )?;

    // Phase 3: Initialize masonry state with configuration ✨ WARNING 11
    let mut masonry_state =
        MasonryTrackState::new_with_tolerance(config.track_count, config.item_tolerance);

    // Phase 4: Collect and sort items by placement order ✨ WARNING 10
    let grid_items = item_collection::collect_and_sort_masonry_items(tree, node_id)?;

    // Phase 5: Place items using pre-sized tracks with optional dense packing
    let mut placed_items = Vec::new();

    for item in grid_items {
        // Grid axis determines which span to use (perpendicular to masonry flow)
        let item_span = match config.masonry_axis {
            AbstractAxis::Block => item.column_span,  // Vertical flow → spans across columns
            AbstractAxis::Inline => item.row_span,    // Horizontal flow → spans across rows
        };

        // Determine placement track based on span and dense packing
        let placement_track = if item_span > 1 {
            // Spanning items ALWAYS need dense placement logic to find shortest span
            // (not just when dense_packing is enabled)
            masonry_state.find_dense_placement(item_span)
                .unwrap_or_else(|| masonry_state.find_shortest_track_with_tolerance())
        } else if config.dense_packing {
            // Single-span item with dense packing: try gap first
            let item_size = item_collection::estimate_item_size_for_masonry(
                tree,
                item.node_id,
                &inputs,
                config.masonry_axis,
            )?;
            let item_masonry_size = match config.masonry_axis {
                AbstractAxis::Block => item_size.height,
                AbstractAxis::Inline => item_size.width,
            };

            let shortest_track = masonry_state.find_shortest_track_with_tolerance();
            let normal_track_size: f32 = (shortest_track..(shortest_track + item_span))
                .map(|i| track_sizes.get(i).copied().unwrap_or(0.0))
                .sum();

            let gaps = gap_detection::detect_compatible_gaps(
                &masonry_state,
                &track_sizes,
                item_span,
                item_masonry_size,
                normal_track_size,
                config.item_tolerance,
            );

            gaps.first()
                .map(|gap| gap.track_index)
                .unwrap_or(shortest_track)
        } else {
            // Single-span item without dense packing: use standard shortest track
            masonry_state.find_shortest_track_with_tolerance()
        };

        // Place item using determined track
        let placement = item_collection::place_item_in_taffy_sized_track(
            tree,
            &item,
            placement_track,
            &track_sizes[placement_track], // Use actual Taffy track size
            &masonry_state,
            config.masonry_axis,
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

    // Phase 5.5: Collapse empty auto-fit tracks if needed
    let mut collapsed_track_sizes = track_sizes.clone();
    if let Some((auto_fit_start, auto_fit_end)) = config.auto_fit_range {
        collapse_auto_fit_tracks_in_range(
            &mut placed_items,
            &mut collapsed_track_sizes,
            auto_fit_start,
            auto_fit_end,
        );
    }

    // Extract gap size for position calculations
    let node = tree.node_from_id(node_id.into());
    let gap_size = if let Some(styles) = node.primary_styles() {
        let style_wrapper = stylo_taffy::TaffyStyloStyle::from(styles);
        use taffy::ResolveOrZero;
        
        // Gap is between tracks on the grid axis (Inline=Horizontal=Width, Block=Vertical=Height)
        match config.grid_axis {
            AbstractAxis::Inline => {
                // Inline=Horizontal → use column gap (width)
                let container_size = inputs.known_dimensions.width.unwrap_or(0.0);
                style_wrapper.gap().width.resolve_or_zero(Some(container_size), |_, _| 0.0)
            }
            AbstractAxis::Block => {
                // Block=Vertical → use row gap (height)
                let container_size = inputs.known_dimensions.height.unwrap_or(0.0);
                style_wrapper.gap().height.resolve_or_zero(Some(container_size), |_, _| 0.0)
            }
        }
    } else {
        0.0
    };

    // Phase 5.7: Layout items first to extract baselines
    let layout_outputs = layout_output::layout_masonry_items(
        tree,
        &placed_items,
        inputs,
        config.masonry_axis,
        &collapsed_track_sizes,
        gap_size,
    )?;

    // Phase 5.8: Calculate baseline alignment adjustments using layout outputs
    let container_size = inputs.known_dimensions;
    let baseline_adjustments = baseline_alignment::calculate_baseline_adjustments(
        tree,
        &placed_items,
        &layout_outputs,  // ✅ Pass layout outputs for baseline extraction
        config.masonry_axis,
        container_size,
    )?;

    // Phase 5.9: Apply baseline adjustments to masonry_axis_position
    for adjustment in baseline_adjustments {
        if let Some((_, grid_area)) = placed_items.get_mut(adjustment.item_index) {
            grid_area.masonry_axis_position += adjustment.position_adjustment;
        }
    }

    // Phase 6: Apply positions to items
    layout_output::apply_masonry_positions(
        tree,
        &placed_items,
        &layout_outputs,
        config.masonry_axis,
        &collapsed_track_sizes,
        gap_size,
    );

    // Phase 7: Generate container layout output
    Ok(layout_output::generate_container_output(
        &placed_items,
        config.masonry_axis,
        inputs,
        &collapsed_track_sizes,
        gap_size,
    ))
}

/// Collapse empty auto-fit tracks in the specified range
/// This implements the CSS Grid auto-fit behavior where empty auto-fit tracks collapse to size 0
fn collapse_auto_fit_tracks_in_range(
    placed_items: &mut Vec<(NodeId, stylo_taffy::GridArea)>,
    track_sizes: &mut Vec<f32>,
    auto_fit_start: usize,
    auto_fit_end: usize,
) {
    use std::collections::HashSet;

    // Step 1: Identify which tracks have items
    // An item occupies all tracks from grid_axis_start to grid_axis_end (exclusive)
    let mut tracks_with_items = HashSet::new();
    for (_node_id, grid_area) in placed_items.iter() {
        for track in grid_area.grid_axis_start..grid_area.grid_axis_end {
            tracks_with_items.insert(track);
        }
    }

    // Step 2: Build list of tracks to collapse (empty auto-fit tracks)
    let mut tracks_to_collapse = Vec::new();
    for idx in auto_fit_start..auto_fit_end {
        if idx < track_sizes.len() && !tracks_with_items.contains(&idx) {
            tracks_to_collapse.push(idx);
        }
    }

    // If no tracks to collapse, we're done
    if tracks_to_collapse.is_empty() {
        return;
    }

    // Step 3: Calculate cumulative offset for each track position
    // This is the total size of all collapsed tracks before this position
    let mut track_position_shifts = vec![0.0; track_sizes.len()];
    let mut cumulative_shift = 0.0;
    
    for idx in 0..track_sizes.len() {
        track_position_shifts[idx] = cumulative_shift;
        
        // If this track should be collapsed, add its size to cumulative shift
        if tracks_to_collapse.contains(&idx) {
            cumulative_shift += track_sizes[idx];
            track_sizes[idx] = 0.0; // Collapse the track to size 0
        }
    }

    // Step 4: Recalculate grid_axis positions for all items
    // For items in the grid axis (horizontal for column masonry), we need to calculate
    // their position based on the sum of track sizes before them
    for (_node_id, grid_area) in placed_items.iter_mut() {
        // Calculate new position by summing non-collapsed track sizes before this item
        let mut new_position = 0.0;
        for idx in 0..grid_area.grid_axis_start {
            if idx < track_sizes.len() {
                new_position += track_sizes[idx];
            }
        }
        
        // Note: GridArea doesn't have a grid_axis_position field to update
        // The position is calculated during apply_masonry_positions using track_sizes
        // Since we've updated track_sizes to have 0 for collapsed tracks,
        // the position calculation will automatically account for collapsed tracks
    }
}
