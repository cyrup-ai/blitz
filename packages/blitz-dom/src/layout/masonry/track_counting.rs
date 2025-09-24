//! Track counting utilities for CSS Grid Level 3 masonry layout
//!
//! Handles counting of tracks in the definite (non-masonry) axis.

use taffy::GridContainerStyle;
use taffy::geometry::AbstractAxis;
use taffy::prelude::NodeId;

use super::super::grid_errors::GridPreprocessingError;
use crate::BaseDocument;

/// Get track count from the definite (non-masonry) axis
#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout
pub fn get_definite_axis_track_count(
    tree: &BaseDocument,
    node_id: NodeId,
    masonry_axis: AbstractAxis,
) -> Result<usize, GridPreprocessingError> {
    let node = tree.node_from_id(node_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "track_count_extraction",
            node_id.into(),
            "Primary styles not available",
        )
    })?;

    let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);

    // Get track count from the definite axis
    let track_count = match masonry_axis {
        AbstractAxis::Block => {
            // Masonry rows: count explicit columns
            style_wrapper
                .grid_template_columns()
                .map(|tracks| tracks.count())
                .unwrap_or(1)
        }
        AbstractAxis::Inline => {
            // Masonry columns: count explicit rows
            style_wrapper
                .grid_template_rows()
                .map(|tracks| tracks.count())
                .unwrap_or(1)
        }
    };

    Ok(track_count.max(1))
}

/// Calculate track count for the definite (non-masonry) axis
#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout
pub fn calculate_definite_track_count(
    tree: &BaseDocument,
    node_id: NodeId,
) -> Result<usize, GridPreprocessingError> {
    let node = tree.node_from_id(node_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "track_count_calculation",
            node_id.into(),
            "Primary styles not available",
        )
    })?;

    let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);

    // Count explicit tracks on definite axis
    let track_count = if style_wrapper.has_masonry_rows() {
        // Masonry rows: count explicit columns
        style_wrapper
            .grid_template_columns()
            .map(|tracks| tracks.count())
            .unwrap_or(1)
    } else {
        // Masonry columns: count explicit rows
        style_wrapper
            .grid_template_rows()
            .map(|tracks| tracks.count())
            .unwrap_or(1)
    };

    Ok(track_count.max(1)) // Ensure at least 1 track
}
