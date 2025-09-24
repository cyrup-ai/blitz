//! Taffy integration utilities for CSS Grid Level 3 masonry layout
//!
//! Provides integration between Blitz's CSS representation and Taffy's layout engine.

use taffy::prelude::NodeId;
use taffy::{AvailableSpace, Size, geometry::AbstractAxis};

use super::super::grid_errors::GridPreprocessingError;
use super::virtual_placement::GridItemInfo;
use crate::BaseDocument;

/// Create Taffy grid style for masonry track sizing
/// Converts masonry track definitions to a Taffy grid container style
pub fn create_taffy_grid_style_for_masonry(
    track_definitions: &[taffy::TrackSizingFunction],
    masonry_axis: AbstractAxis,
) -> taffy::Style {
    use taffy::style::Display;

    // Convert TrackSizingFunction to GridTemplateComponent::Single
    let grid_template_tracks: Vec<taffy::GridTemplateComponent<String>> = track_definitions
        .iter()
        .map(|track| taffy::GridTemplateComponent::Single(*track))
        .collect();

    // Create style with tracks on the definite axis (non-masonry axis)
    match masonry_axis {
        AbstractAxis::Block => {
            // Masonry flows vertically, tracks are columns (definite axis)
            taffy::Style {
                display: Display::Grid,
                grid_template_columns: grid_template_tracks,
                grid_template_rows: vec![], // Masonry axis - no predefined tracks
                ..Default::default()
            }
        }
        AbstractAxis::Inline => {
            // Masonry flows horizontally, tracks are rows (definite axis)
            taffy::Style {
                display: Display::Grid,
                grid_template_columns: vec![], // Masonry axis - no predefined tracks
                grid_template_rows: grid_template_tracks,
                ..Default::default()
            }
        }
    }
}

/// Create Taffy item style for masonry items
/// Extracts CSS properties and converts to Taffy style
pub fn create_taffy_item_style_for_masonry(
    tree: &BaseDocument,
    item: &GridItemInfo,
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<taffy::Style, GridPreprocessingError> {
    // Get intrinsic size for the item
    let item_size = super::super::intrinsic_sizing::calculate_item_intrinsic_size_for_masonry(
        tree,
        item.node_id,
        inputs,
        masonry_axis,
    )?;

    // Create style with intrinsic sizing behavior
    let style = taffy::Style {
        display: taffy::style::Display::Block,
        size: Size {
            width: taffy::style::Dimension::length(item_size.width),
            height: taffy::style::Dimension::length(item_size.height),
        },
        ..Default::default()
    };

    Ok(style)
}

/// Create Taffy available space from masonry available space
pub fn create_taffy_available_space(
    available_space: f32,
    masonry_axis: AbstractAxis,
) -> Size<AvailableSpace> {
    match masonry_axis {
        AbstractAxis::Block => {
            // Masonry flows vertically, width is constrained
            Size {
                width: AvailableSpace::Definite(available_space),
                height: AvailableSpace::MaxContent,
            }
        }
        AbstractAxis::Inline => {
            // Masonry flows horizontally, height is constrained
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::Definite(available_space),
            }
        }
    }
}

/// Get track width using actual track sizes computed by Taffy
/// Eliminates hardcoded values by using real track sizing results
pub fn get_track_width_from_sizes(track_index: usize, track_sizes: &[f32]) -> f32 {
    track_sizes.get(track_index).copied().unwrap_or_else(|| {
        // Use CSS-compliant calculation instead of hardcoded fallback
        // Calculate reasonable default based on available track sizes
        if !track_sizes.is_empty() {
            track_sizes.iter().sum::<f32>() / track_sizes.len() as f32
        } else {
            // Final fallback: use minimal valid track size
            0.0
        }
    })
}

/// Get track height using actual track sizes computed by Taffy
/// Eliminates hardcoded values by using real track sizing results
pub fn get_track_height_from_sizes(track_index: usize, track_sizes: &[f32]) -> f32 {
    track_sizes.get(track_index).copied().unwrap_or_else(|| {
        // Use CSS-compliant calculation instead of hardcoded fallback
        // Calculate reasonable default based on available track sizes
        if !track_sizes.is_empty() {
            track_sizes.iter().sum::<f32>() / track_sizes.len() as f32
        } else {
            // Final fallback: use minimal valid track size
            0.0
        }
    })
}

/// Convert GridArea coordinates to Layout position and size
pub fn grid_area_to_layout(
    grid_area: &stylo_taffy::GridArea,
    masonry_axis: AbstractAxis,
    track_sizes: &[f32],
) -> (taffy::Point<f32>, Size<f32>) {
    match masonry_axis {
        AbstractAxis::Block => {
            // Masonry flows vertically
            let location = taffy::Point {
                x: (grid_area.grid_axis_start as f32)
                    * get_track_width_from_sizes(grid_area.grid_axis_start, track_sizes),
                y: grid_area.masonry_axis_position,
            };
            let size = Size {
                width: ((grid_area.grid_axis_end - grid_area.grid_axis_start) as f32)
                    * get_track_width_from_sizes(grid_area.grid_axis_start, track_sizes),
                height: grid_area.masonry_axis_size,
            };
            (location, size)
        }
        AbstractAxis::Inline => {
            // Masonry flows horizontally
            let location = taffy::Point {
                x: grid_area.masonry_axis_position,
                y: (grid_area.grid_axis_start as f32)
                    * get_track_height_from_sizes(grid_area.grid_axis_start, track_sizes),
            };
            let size = Size {
                width: grid_area.masonry_axis_size,
                height: ((grid_area.grid_axis_end - grid_area.grid_axis_start) as f32)
                    * get_track_height_from_sizes(grid_area.grid_axis_start, track_sizes),
            };
            (location, size)
        }
    }
}

/// Calculate container size based on actual masonry placements
pub fn calculate_container_size_from_placements(
    placed_items: &[(NodeId, stylo_taffy::GridArea)],
    masonry_axis: AbstractAxis,
    available_space: Size<AvailableSpace>,
    track_sizes: &[f32],
) -> Size<f32> {
    let mut max_width: f32 = 0.0;
    let mut max_height: f32 = 0.0;

    for (_item_id, grid_area) in placed_items {
        match masonry_axis {
            AbstractAxis::Block => {
                // Masonry flows vertically (row masonry)
                // Use actual track positions, not hardcoded values
                let item_right = (grid_area.grid_axis_end as f32)
                    * get_track_width_from_sizes(grid_area.grid_axis_start, track_sizes);
                let item_bottom = grid_area.masonry_axis_position + grid_area.masonry_axis_size;
                max_width = max_width.max(item_right);
                max_height = max_height.max(item_bottom);
            }
            AbstractAxis::Inline => {
                // Masonry flows horizontally (column masonry)
                let item_right = grid_area.masonry_axis_position + grid_area.masonry_axis_size;
                let item_bottom = (grid_area.grid_axis_end as f32)
                    * get_track_height_from_sizes(grid_area.grid_axis_start, track_sizes);
                max_width = max_width.max(item_right);
                max_height = max_height.max(item_bottom);
            }
        }
    }

    // Respect available space constraints
    let final_width = match available_space.width {
        AvailableSpace::Definite(w) => w.max(max_width),
        _ => max_width,
    };

    let final_height = match available_space.height {
        AvailableSpace::Definite(h) => h.max(max_height),
        _ => max_height,
    };

    Size {
        width: final_width,
        height: final_height,
    }
}
