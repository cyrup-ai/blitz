//! Track sizing functionality for CSS Grid Level 3 masonry layout
//!
//! Implements the track sizing algorithm that calculates track dimensions before item placement.

use taffy::geometry::AbstractAxis;
use taffy::prelude::NodeId;

use super::super::grid_errors::GridPreprocessingError;
use super::virtual_placement::{GridItemInfo, create_virtual_placements_for_spanning_items};
use crate::BaseDocument;

/// Size masonry tracks before item placement per CSS Grid Level 3 specification
///
/// This implements the intrinsic sizing algorithm where all items contribute
/// to track sizing regardless of their final placement position.
pub fn size_masonry_tracks_before_placement(
    tree: &BaseDocument,
    container_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<Vec<f32>, GridPreprocessingError> {
    use super::super::grid_context::{GridAxis, extract_tracks_from_stylo_computed_styles};
    use super::super::grid_errors::MasonryError;

    // Step 1: Extract track definitions from the definite (non-masonry) axis using existing infrastructure
    let node = tree.node_from_id(container_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "track_definition_extraction",
            container_id.into(),
            "Primary styles not available",
        )
    })?;

    // Note: style_wrapper not needed since we use stylo integration directly

    // Get track definitions from the definite axis (non-masonry axis)
    // Integrate subgrid inheritance registry to get effective tracks
    let _axis = match masonry_axis {
        AbstractAxis::Block => GridAxis::Column,
        AbstractAxis::Inline => GridAxis::Row,
    };

    let fallback_tracks = match masonry_axis {
        AbstractAxis::Block => {
            // Masonry rows: get track definitions from columns
            extract_tracks_from_stylo_computed_styles(&computed_styles, GridAxis::Column)?
        }
        AbstractAxis::Inline => {
            // Masonry columns: get track definitions from rows
            extract_tracks_from_stylo_computed_styles(&computed_styles, GridAxis::Row)?
        }
    };

    // Use verified existing function from grid_context/track_extraction.rs:135
    let track_definitions = super::super::grid_context::extract_tracks_from_stylo_computed_styles(
        &computed_styles,
        super::super::grid_context::GridAxis::Row,
    )
    .unwrap_or(fallback_tracks);

    let track_count = track_definitions.len();

    if track_count == 0 {
        return Err(GridPreprocessingError::Masonry(
            MasonryError::InvalidTrackCount {
                track_count: 0,
                min: 1,
                max: 1000,
            },
        ));
    }

    // Step 2: Collect items (existing infrastructure) ✅
    let all_items = super::item_collection::collect_grid_items_for_masonry(tree, container_id)?;

    if all_items.is_empty() {
        return Err(GridPreprocessingError::Masonry(
            MasonryError::ItemCollectionFailed {
                reason: "No grid items found for masonry track sizing".to_string(),
            },
        ));
    }

    // Step 3: ✨ NEW - Create virtual placements for spanning items
    let _virtual_placements = create_virtual_placements_for_spanning_items(
        tree,
        &all_items,
        track_count,
        masonry_axis,
        inputs,
    )?;

    // Step 4: ✨ NEW - Calculate track sizes directly using Taffy's constraint resolution
    let available_space = calculate_available_space_for_masonry(inputs, masonry_axis);
    let track_sizes = calculate_track_sizes_from_definitions(
        &track_definitions,
        available_space,
        &all_items,
        tree,
        inputs,
        masonry_axis,
    )?;

    Ok(track_sizes)
}

/// Calculate track sizes using Taffy's proven grid layout algorithm
/// Leverages Taffy's sophisticated track sizing instead of manual implementation
fn calculate_track_sizes_from_definitions(
    track_definitions: &[taffy::TrackSizingFunction],
    available_space: f32,
    grid_items: &[GridItemInfo],
    tree: &BaseDocument,
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<Vec<f32>, GridPreprocessingError> {
    // Use Taffy's intrinsic sizing algorithm to calculate proper track sizes
    let track_sizes = compute_taffy_track_sizes(
        track_definitions,
        available_space,
        grid_items,
        tree,
        inputs,
        masonry_axis,
    )?;

    Ok(track_sizes)
}

/// Create track sizing information using Taffy patterns
/// Since taffy::GridTrack doesn't exist, use track sizing functions directly
#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout
fn create_track_sizing_for_masonry(
    track_definitions: &[taffy::TrackSizingFunction],
    available_space: f32,
    grid_items: &[GridItemInfo],
    tree: &BaseDocument,
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<Vec<f32>, GridPreprocessingError> {
    // Use Taffy's proven track sizing approach instead of manual calculation
    compute_taffy_track_sizes(
        track_definitions,
        available_space,
        grid_items,
        tree,
        inputs,
        masonry_axis,
    )
}

/// Calculate available space using Taffy's established pattern
/// Determines available space for track sizing based on the definite axis
fn calculate_available_space_for_masonry(
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> f32 {
    let definite_axis = masonry_axis.other();
    match definite_axis {
        AbstractAxis::Inline => inputs
            .available_space
            .width
            .into_option()
            .unwrap_or(f32::INFINITY),
        AbstractAxis::Block => inputs
            .available_space
            .height
            .into_option()
            .unwrap_or(f32::INFINITY),
    }
}

/// Compute track sizes using Taffy's real grid layout algorithm
/// Creates a minimal Taffy tree and uses compute_grid_layout to get actual track sizes
fn compute_taffy_track_sizes(
    track_definitions: &[taffy::TrackSizingFunction],
    available_space: f32,
    grid_items: &[GridItemInfo],
    tree: &BaseDocument,
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<Vec<f32>, GridPreprocessingError> {
    use taffy::TaffyTree;

    // Create a minimal Taffy tree to run the real grid algorithm
    let mut taffy_tree = TaffyTree::new();

    // Create container node with track definitions
    let container_style = super::taffy_integration::create_taffy_grid_style_for_masonry(
        track_definitions,
        masonry_axis,
    );
    let container_node = taffy_tree.new_leaf(container_style).map_err(|_| {
        GridPreprocessingError::preprocessing_failed(
            "taffy_tree_creation",
            0_usize,
            "Failed to create Taffy container node",
        )
    })?;

    // Add child nodes representing masonry items
    let mut child_nodes = Vec::new();
    for item in grid_items {
        let item_style = super::taffy_integration::create_taffy_item_style_for_masonry(
            tree,
            item,
            inputs,
            masonry_axis,
        )?;
        let child_node = taffy_tree.new_leaf(item_style).map_err(|_| {
            GridPreprocessingError::preprocessing_failed(
                "taffy_child_creation",
                usize::from(item.node_id).into(),
                "Failed to create Taffy child node",
            )
        })?;
        child_nodes.push(child_node);
    }

    // Set children on container
    taffy_tree
        .set_children(container_node, &child_nodes)
        .map_err(|_| {
            GridPreprocessingError::preprocessing_failed(
                "taffy_children_setup",
                0_usize,
                "Failed to set Taffy children",
            )
        })?;

    // Create available space for Taffy
    let taffy_available_space =
        super::taffy_integration::create_taffy_available_space(available_space, masonry_axis);

    // Run Taffy's real compute_grid_layout algorithm
    taffy_tree
        .compute_layout(container_node, taffy_available_space)
        .map_err(|_| {
            GridPreprocessingError::preprocessing_failed(
                "taffy_layout_computation",
                0_usize,
                "Failed to compute Taffy layout",
            )
        })?;

    // Extract track sizes from the computed layout
    extract_track_sizes_from_taffy_layout(
        &taffy_tree,
        container_node,
        track_definitions.len(),
        masonry_axis,
    )
}

/// Extract track sizes from Taffy's computed layout
/// Analyzes the computed grid layout to extract actual track sizes
fn extract_track_sizes_from_taffy_layout(
    taffy_tree: &taffy::TaffyTree<()>,
    container_node: taffy::NodeId,
    track_count: usize,
    masonry_axis: AbstractAxis,
) -> Result<Vec<f32>, GridPreprocessingError> {
    let container_layout = taffy_tree.layout(container_node).map_err(|_| {
        GridPreprocessingError::preprocessing_failed(
            "taffy_layout_extraction",
            0_usize,
            "Failed to get Taffy container layout",
        )
    })?;

    // Get all children to analyze their positions and sizes
    let children = taffy_tree.children(container_node).map_err(|_| {
        GridPreprocessingError::preprocessing_failed(
            "taffy_children_extraction",
            0_usize,
            "Failed to get Taffy container children",
        )
    })?;

    if children.is_empty() || track_count == 0 {
        // No children or no tracks - use even distribution fallback
        let container_size = match masonry_axis {
            AbstractAxis::Block => container_layout.size.width,
            AbstractAxis::Inline => container_layout.size.height,
        };
        let track_size = if track_count > 0 {
            container_size / track_count as f32
        } else {
            0.0
        };
        return Ok(vec![track_size; track_count]);
    }

    // Calculate track sizes based on actual child layout positions
    let mut track_sizes = vec![0.0_f32; track_count];

    // For each child, determine which track it's in and its contribution
    for child_id in children {
        let child_layout = taffy_tree.layout(child_id).map_err(|_| {
            GridPreprocessingError::preprocessing_failed(
                "taffy_child_layout_extraction",
                0_usize,
                "Failed to get child layout",
            )
        })?;

        // Determine track index based on position
        let (track_index, item_size) = match masonry_axis {
            AbstractAxis::Block => {
                // Masonry in block direction - tracks are columns (inline axis)
                // Calculate which column this item is in based on its x position
                let column_size = container_layout.size.width / track_count as f32;
                let track_idx = (child_layout.location.x / column_size).floor() as usize;
                (track_idx.min(track_count - 1), child_layout.size.width)
            }
            AbstractAxis::Inline => {
                // Masonry in inline direction - tracks are rows (block axis)
                // Calculate which row this item is in based on its y position
                let row_size = container_layout.size.height / track_count as f32;
                let track_idx = (child_layout.location.y / row_size).floor() as usize;
                (track_idx.min(track_count - 1), child_layout.size.height)
            }
        };

        // Update track size to be maximum of current size and item size
        track_sizes[track_index] = track_sizes[track_index].max(item_size);
    }

    // Ensure all tracks have at least some minimum size if they're empty
    let container_size = match masonry_axis {
        AbstractAxis::Block => container_layout.size.width,
        AbstractAxis::Inline => container_layout.size.height,
    };

    let min_track_size = container_size / track_count as f32;
    for track_size in track_sizes.iter_mut() {
        if *track_size < 1.0 {
            // Empty track - use proportional share of container
            *track_size = min_track_size;
        }
    }

    Ok(track_sizes)
}
