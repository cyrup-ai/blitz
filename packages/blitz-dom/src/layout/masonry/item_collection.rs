//! Item collection and placement for CSS Grid Level 3 masonry layout
//!
//! Handles collection of grid items and their placement within masonry tracks.

use taffy::prelude::NodeId;
use taffy::{
    CoreStyle, GridContainerStyle, GridItemStyle, Size, TraversePartialTree, geometry::AbstractAxis,
};

use super::super::grid_errors::GridPreprocessingError;
use super::super::intrinsic_sizing::calculate_item_intrinsic_size_for_masonry;
use super::track_counting::grid_axis_from_masonry;
use super::virtual_placement::GridItemInfo;
use crate::BaseDocument;

/// Enhanced track configuration with item-tolerance support
/// Provides complete configuration for CSS Grid Level 3 masonry layout
#[derive(Debug, Clone)]
pub struct MasonryConfig {
    /// Direction items flow/cascade (masonry axis)
    pub masonry_axis: AbstractAxis,
    /// Direction tracks are counted (grid axis - perpendicular to masonry_axis)
    pub grid_axis: AbstractAxis,
    pub track_count: usize,
    pub item_tolerance: f32,
    pub dense_packing: bool,
    /// Range of auto-fit tracks (start_index, end_index) if auto-fit is used
    pub auto_fit_range: Option<(usize, usize)>,
}

/// Calculate track count and extract item-tolerance from computed styles
/// Implements CSS Grid Level 3 masonry configuration extraction
pub fn calculate_masonry_config(
    tree: &BaseDocument,
    node_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
) -> Result<MasonryConfig, GridPreprocessingError> {
    let node = tree.node_from_id(node_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "masonry_config_calculation",
            node_id.into(),
            "Primary styles not available",
        )
    })?;

    let style_wrapper = stylo_taffy::TaffyStyloStyle::from(&*computed_styles);

    // Determine masonry axis by checking RAW stylo values before conversion
    let raw_rows = style_wrapper.raw_grid_template_rows();
    let raw_cols = style_wrapper.raw_grid_template_columns();
    
    let mut has_masonry_rows = stylo_taffy::convert::is_masonry_axis(raw_rows);
    let mut has_masonry_columns = stylo_taffy::convert::is_masonry_axis(raw_cols);
    
    // Check for display: masonry or display: inline-masonry
    let display = computed_styles.clone_display();
    let display_is_masonry = stylo_taffy::convert::is_display_masonry(display);
    
    // WORKAROUND: Infer masonry axis when not explicitly declared
    // This applies to both display:masonry and display:grid with masonry features
    // If one axis has explicit tracks and the other doesn't, the empty one is masonry
    if !has_masonry_rows && !has_masonry_columns {
        let cols_has_tracks = style_wrapper.grid_template_columns().is_some();
        let rows_has_tracks = style_wrapper.grid_template_rows().is_some();
        
        if cols_has_tracks && !rows_has_tracks {
            // Columns defined, rows empty → rows are masonry
            has_masonry_rows = true;
        } else if rows_has_tracks && !cols_has_tracks {
            // Rows defined, columns empty → columns are masonry
            has_masonry_columns = true;
        } else if display_is_masonry {
            // display: masonry with neither axis defined → default to rows as masonry
            has_masonry_rows = true;
        }
    }
    
    // Determine masonry flow axis (direction items stack/cascade):
    // - has_masonry_rows: rows are masonry → items flow DOWN (vertically) → Block axis
    // - has_masonry_columns: columns are masonry → items flow ACROSS (horizontally) → Inline axis
    // Note: grid axis (where tracks are counted) is perpendicular to masonry axis
    let masonry_axis = if has_masonry_rows {
        AbstractAxis::Block   // Rows are masonry: items flow DOWN (vertical)
    } else {
        AbstractAxis::Inline  // Columns are masonry: items flow ACROSS (horizontal)
    };
    
    // Grid axis is perpendicular to masonry axis (use shared helper)
    let grid_axis = grid_axis_from_masonry(masonry_axis);

    // Extract available size for grid axis (Inline=Horizontal=Width, Block=Vertical=Height)
    // Try known_dimensions first, then fall back to available_space for definite values
    let available_size = match grid_axis {
        AbstractAxis::Inline => {
            // Inline=Horizontal → need width for column spacing
            inputs.known_dimensions.width
                .or_else(|| inputs.available_space.width.into_option())
        }
        AbstractAxis::Block => {
            // Block=Vertical → need height for row spacing
            inputs.known_dimensions.height
                .or_else(|| inputs.available_space.height.into_option())
        }
    };

    // Check if auto-repeat exists to get both count and auto-fit range
    // Get tracks from grid axis (Inline=Horizontal=Columns, Block=Vertical=Rows)
    let tracks = match grid_axis {
        AbstractAxis::Inline => style_wrapper.grid_template_columns(),  // Inline=Horizontal → columns
        AbstractAxis::Block => style_wrapper.grid_template_rows(),      // Block=Vertical → rows
    };
    
    let (final_track_count, auto_fit_range) = if let Some(tracks) = tracks {
        let has_auto = tracks.clone().any(|component| match component {
            taffy::GenericGridTemplateComponent::Repeat(repeat) => {
                matches!(repeat.count, taffy::RepetitionCount::AutoFill | taffy::RepetitionCount::AutoFit)
            }
            _ => false,
        });
        
        if has_auto {
            // Get full TrackCountResult with auto-fit range
            let result = super::track_counting::calculate_auto_repeat_track_count(
                tree,
                node_id,
                masonry_axis,
                available_size,
            )?;
            (result.count, result.auto_fit_range)
        } else {
            // No auto-repeat, use simple count
            let count = super::track_counting::get_definite_axis_track_count(
                tree,
                node_id,
                masonry_axis,
                available_size,
            )?;
            (count, None)
        }
    } else {
        (1, None)
    };

    // Extract item-tolerance from computed styles using CSS Grid Level 3 properties
    let item_tolerance = extract_masonry_item_tolerance_from_styles(tree, node_id)?;

    // Extract grid-auto-flow to detect dense keyword
    let dense_packing = extract_dense_packing_from_styles(tree, node_id)?;

    Ok(MasonryConfig {
        masonry_axis,
        grid_axis,
        track_count: final_track_count.max(1), // Ensure at least 1 track
        item_tolerance,
        dense_packing,
        auto_fit_range,
    })
}

/// Extract masonry item tolerance from computed styles
/// Implements CSS Grid Level 3 masonry-item-tolerance property extraction
fn extract_masonry_item_tolerance_from_styles(
    tree: &BaseDocument,
    node_id: NodeId,
) -> Result<f32, GridPreprocessingError> {
    let node = tree.node_from_id(node_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "masonry_item_tolerance_extraction",
            node_id.into(),
            "Primary styles not available",
        )
    })?;

    // Extract masonry-item-tolerance property from computed styles
    // CSS Grid Level 3 spec default: 1em (browser's root font size)
    let font_size = computed_styles.clone_font_size().used_size().px();
    let tolerance = font_size; // Use actual font-size in pixels (1em equivalent)
    Ok(tolerance)
}

/// Extract dense packing configuration from grid-auto-flow
/// Implements CSS Grid Level 3 dense packing detection
fn extract_dense_packing_from_styles(
    tree: &BaseDocument,
    node_id: NodeId,
) -> Result<bool, GridPreprocessingError> {
    let node = tree.node_from_id(node_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "dense_packing_extraction",
            node_id.into(),
            "Primary styles not available",
        )
    })?;

    let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);
    let grid_auto_flow = style_wrapper.grid_auto_flow();

    // Check for Dense variant using existing Taffy GridAutoFlow enum
    Ok(matches!(
        grid_auto_flow,
        taffy::GridAutoFlow::RowDense | taffy::GridAutoFlow::ColumnDense
    ))
}

/// Enhanced item collection that processes spans for intrinsic sizing
///
/// Uses existing GridItemInfo fields that are currently unused (WARNING 10)
pub fn collect_and_sort_masonry_items(
    tree: &BaseDocument,
    container_id: NodeId,
) -> Result<Vec<GridItemInfo>, GridPreprocessingError> {
    let mut items = collect_grid_items_for_masonry(tree, container_id)?; // Existing function

    // Sort by order field for proper placement sequence ✨ Uses WARNING 10 field
    items.sort_by_key(|item| item.order);

    Ok(items)
}

/// Collect grid items that need masonry placement
/// Enhanced to detect grid spans and maintain proper placement order
pub fn collect_grid_items_for_masonry(
    tree: &BaseDocument,
    container_id: NodeId,
) -> Result<Vec<GridItemInfo>, GridPreprocessingError> {
    let mut items = Vec::new();
    let child_count = tree.child_count(container_id);

    for i in 0..child_count {
        let child_id = tree.get_child_id(container_id, i);

        // Check if child is a grid item (not absolutely positioned)
        let node = tree.node_from_id(child_id.into());
        if let Some(styles) = node.primary_styles() {
            let style_wrapper = stylo_taffy::TaffyStyloStyle::from(styles);

            // Skip absolutely positioned items
            if style_wrapper.position() != taffy::Position::Absolute {
                // Extract grid placement information for enhanced masonry placement
                let grid_row = style_wrapper.grid_row();
                let grid_column = style_wrapper.grid_column();

                // Enhanced span calculation handling all GridPlacement variants
                let row_span = match (grid_row.start, grid_row.end) {
                    (taffy::GridPlacement::Line(start), taffy::GridPlacement::Line(end)) => {
                        (end.as_i16() - start.as_i16()).abs().max(1) as usize
                    }
                    (taffy::GridPlacement::Line(_), taffy::GridPlacement::Span(span)) => {
                        span as usize
                    }
                    (taffy::GridPlacement::Span(span), taffy::GridPlacement::Line(_)) => {
                        span as usize
                    }
                    (taffy::GridPlacement::Span(span), _) => span as usize,
                    (_, taffy::GridPlacement::Span(span)) => span as usize,
                    (taffy::GridPlacement::NamedSpan(_, span), _) => span as usize,
                    (_, taffy::GridPlacement::NamedSpan(_, span)) => span as usize,
                    _ => 1, // Auto, NamedLine, or invalid combinations default to 1
                };

                let column_span = match (grid_column.start, grid_column.end) {
                    (taffy::GridPlacement::Line(start), taffy::GridPlacement::Line(end)) => {
                        (end.as_i16() - start.as_i16()).abs().max(1) as usize
                    }
                    (taffy::GridPlacement::Line(_), taffy::GridPlacement::Span(span)) => {
                        span as usize
                    }
                    (taffy::GridPlacement::Span(span), taffy::GridPlacement::Line(_)) => {
                        span as usize
                    }
                    (taffy::GridPlacement::Span(span), _) => span as usize,
                    (_, taffy::GridPlacement::Span(span)) => span as usize,
                    (taffy::GridPlacement::NamedSpan(_, span), _) => span as usize,
                    (_, taffy::GridPlacement::NamedSpan(_, span)) => span as usize,
                    _ => 1, // Auto, NamedLine, or invalid combinations default to 1
                };

                items.push(GridItemInfo {
                    node_id: child_id,
                    order: i, // Maintain source order for masonry
                    row_span,
                    column_span,
                });
            }
        }
    }

    Ok(items)
}

/// Calculate masonry item size using proper CSS intrinsic sizing
/// Replaces hardcoded 200.0px/100.0px fallbacks with CSS Sizing Module Level 3 compliance
pub fn estimate_item_size_for_masonry(
    tree: &BaseDocument,
    item_id: NodeId,
    inputs: &taffy::tree::LayoutInput,
    masonry_axis: AbstractAxis,
) -> Result<Size<f32>, GridPreprocessingError> {
    // Use proper intrinsic sizing instead of hardcoded fallbacks
    calculate_item_intrinsic_size_for_masonry(
        tree,
        item_id,
        inputs,
        masonry_axis, // Use actual masonry axis from config
    )
}

/// Place item using Taffy-sized track information
/// Uses actual track sizes from Taffy's track sizing algorithm instead of hardcoded values
pub fn place_item_in_taffy_sized_track(
    tree: &BaseDocument,
    item: &GridItemInfo,
    track_index: usize,
    track_size: &f32,
    masonry_state: &stylo_taffy::MasonryTrackState,
    masonry_axis: AbstractAxis,
    inputs: &taffy::tree::LayoutInput,
) -> Result<(NodeId, stylo_taffy::GridArea), GridPreprocessingError> {
    use stylo_taffy::GridArea;

    // For masonry, items ALWAYS fill their track in the grid axis
    // Only the masonry axis dimension varies based on content
    let span = match masonry_axis {
        AbstractAxis::Block => item.column_span,  // Vertical flow → spans columns
        AbstractAxis::Inline => item.row_span,    // Horizontal flow → spans rows
    };
    let grid_axis_size = track_size * span as f32;
    
    // Get masonry axis size from intrinsic sizing
    let item_size = estimate_item_size_for_masonry(tree, item.node_id, inputs, masonry_axis)?;
    let masonry_axis_size = match masonry_axis {
        AbstractAxis::Block => item_size.height,   // Vertical flow → height varies
        AbstractAxis::Inline => item_size.width,   // Horizontal flow → width varies
    };
    
    // Combine: grid axis always equals track, masonry axis from content
    let constrained_item_size = match masonry_axis {
        AbstractAxis::Block => Size {
            width: grid_axis_size,      // Always fill track width
            height: masonry_axis_size,  // Height from content
        },
        AbstractAxis::Inline => Size {
            width: masonry_axis_size,   // Width from content
            height: grid_axis_size,     // Always fill track height
        },
    };

    // Create placement information using Taffy-sized tracks
    // Grid axis determines which span to use (perpendicular to masonry flow)
    let grid_area = match masonry_axis {
        AbstractAxis::Block => GridArea {
            grid_axis_start: track_index,
            grid_axis_end: track_index + item.column_span, // Vertical flow → spans across columns
            masonry_axis_position: masonry_state.get_track_position(track_index),
            masonry_axis_size: constrained_item_size.height, // Item size in masonry axis
        },
        AbstractAxis::Inline => GridArea {
            grid_axis_start: track_index,
            grid_axis_end: track_index + item.row_span, // Horizontal flow → spans across rows
            masonry_axis_position: masonry_state.get_track_position(track_index),
            masonry_axis_size: constrained_item_size.width, // Item size in masonry axis
        },
    };

    Ok((item.node_id, grid_area))
}

/// Apply track size constraints to item sizing
/// Ensures items respect track boundaries in the definite axis while maintaining aspect ratios
pub fn apply_track_size_constraints(
    item_size: Size<f32>,
    track_size: f32,
    masonry_axis: AbstractAxis,
    item: &GridItemInfo,
) -> Size<f32> {
    match masonry_axis {
        AbstractAxis::Block => {
            // Masonry flows vertically, constrain width to track size
            let span = item.column_span as f32;
            let max_width = track_size * span;

            if item_size.width > max_width {
                // Scale down proportionally to fit within track bounds
                let scale_factor = max_width / item_size.width;
                Size {
                    width: max_width,
                    height: item_size.height * scale_factor,
                }
            } else {
                item_size
            }
        }
        AbstractAxis::Inline => {
            // Masonry flows horizontally, constrain height to track size
            let span = item.row_span as f32;
            let max_height = track_size * span;

            if item_size.height > max_height {
                // Scale down proportionally to fit within track bounds
                let scale_factor = max_height / item_size.height;
                Size {
                    width: item_size.width * scale_factor,
                    height: max_height,
                }
            } else {
                item_size
            }
        }
    }
}
