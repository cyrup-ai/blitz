//! Track counting utilities for CSS Grid Level 3 masonry layout
//!
//! Handles counting of tracks in the definite (non-masonry) axis.

use taffy::{GridContainerStyle, ResolveOrZero};
use taffy::geometry::AbstractAxis;
use taffy::prelude::NodeId;
use taffy::RepetitionCount;

use super::super::grid_errors::GridPreprocessingError;
use crate::BaseDocument;

/// Detect if grid template contains auto-fill or auto-fit
/// Returns the type of auto-repeat found, or None if no auto-repeat exists
fn has_auto_repeat_tracks<'a, I>(tracks: I) -> Option<RepetitionCount>
where
    I: Iterator<Item = taffy::GenericGridTemplateComponent<String, &'a taffy::GridTemplateRepetition<String>>> + Clone,
{
    tracks
        .filter_map(|component| match component {
            taffy::GenericGridTemplateComponent::Single(_) => None,
            taffy::GenericGridTemplateComponent::Repeat(repeat) => {
                match repeat.count {
                    RepetitionCount::AutoFill => Some(RepetitionCount::AutoFill),
                    RepetitionCount::AutoFit => Some(RepetitionCount::AutoFit),
                    RepetitionCount::Count(_) => None,
                }
            }
        })
        .next()
}

/// Calculate actual track count for auto-fill/auto-fit in masonry grid axis
/// Based on Taffy's explicit_grid.rs implementation (lines 103-179)
pub fn calculate_auto_repeat_track_count(
    tree: &BaseDocument,
    node_id: NodeId,
    masonry_axis: AbstractAxis,
    available_size: Option<f32>,
) -> Result<usize, GridPreprocessingError> {
    let node = tree.node_from_id(node_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "auto_repeat_calculation",
            node_id.into(),
            "Primary styles not available",
        )
    })?;

    let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);

    // Get grid axis tracks (opposite of masonry axis)
    let tracks = match masonry_axis {
        AbstractAxis::Block => style_wrapper.grid_template_columns(),
        AbstractAxis::Inline => style_wrapper.grid_template_rows(),
    };

    let Some(tracks) = tracks else {
        return Ok(1);
    };

    // Check for auto-repeat
    let auto_repeat_type = has_auto_repeat_tracks(tracks.clone());
    if auto_repeat_type.is_none() {
        // No auto-repeat, count normally
        return Ok(tracks.count());
    }

    // If no available size, default to 1 repetition
    let Some(container_size) = available_size else {
        return Ok(count_tracks_with_one_auto_repetition(tracks));
    };

    // Calculate non-auto-repeating track count
    let non_auto_count: u16 = tracks.clone()
        .map(|track_def| match track_def {
            taffy::GenericGridTemplateComponent::Single(_) => 1,
            taffy::GenericGridTemplateComponent::Repeat(repeat) => match repeat.count {
                RepetitionCount::Count(count) => count * (repeat.tracks.len() as u16),
                RepetitionCount::AutoFill | RepetitionCount::AutoFit => 0,
            },
        })
        .sum();

    // Find auto-repeat definition
    let auto_repeat = tracks.clone()
        .find_map(|def| match def {
            taffy::GenericGridTemplateComponent::Single(_) => None,
            taffy::GenericGridTemplateComponent::Repeat(repeat) => match repeat.count {
                RepetitionCount::AutoFill | RepetitionCount::AutoFit => Some(repeat),
                _ => None,
            },
        });

    // If no auto-repeat found (shouldn't happen since we checked), default to 1
    let Some(auto_repeat) = auto_repeat else {
        return Ok(count_tracks_with_one_auto_repetition(tracks));
    };

    let repeat_track_count = auto_repeat.tracks.len() as u16;

    // Calculate space used by non-repeating tracks
    let non_repeating_space: f32 = calculate_track_space(tracks.clone(), (), container_size, true);

    // Calculate space per repetition
    let per_repetition_space: f32 = auto_repeat.tracks.iter()
        .map(|sizing_fn| estimate_track_size(*sizing_fn, container_size))
        .sum();

    // Get gap size
    let gap_size = match masonry_axis {
        AbstractAxis::Block => {
            // Masonry rows → columns have horizontal gap
            style_wrapper.gap().width.resolve_or_zero(Some(container_size), |_, _| 0.0)
        }
        AbstractAxis::Inline => {
            // Masonry columns → rows have vertical gap
            style_wrapper.gap().height.resolve_or_zero(Some(container_size), |_, _| 0.0)
        }
    };

    // Calculate first repetition size (includes non-repeating tracks)
    let first_repetition_size = non_repeating_space +
                                per_repetition_space +
                                ((non_auto_count + repeat_track_count).saturating_sub(1) as f32 * gap_size);

    if first_repetition_size > container_size {
        return Ok((non_auto_count + repeat_track_count) as usize);
    }

    // Calculate additional repetitions
    let per_repetition_gap = (repeat_track_count as f32) * gap_size;
    let per_repetition_total = per_repetition_space + per_repetition_gap;

    let additional_reps = ((container_size - first_repetition_size) / per_repetition_total).floor() as u16;
    let total_repetitions = additional_reps + 1;

    Ok((non_auto_count + (repeat_track_count * total_repetitions)) as usize)
}

/// Count tracks assuming 1 auto-repetition when container size unknown
fn count_tracks_with_one_auto_repetition<'a, I>(tracks: I) -> usize
where
    I: Iterator<Item = taffy::GenericGridTemplateComponent<String, &'a taffy::GridTemplateRepetition<String>>> + Clone,
{
    tracks
        .map(|track_def| match track_def {
            taffy::GenericGridTemplateComponent::Single(_) => 1,
            taffy::GenericGridTemplateComponent::Repeat(repeat) => match repeat.count {
                RepetitionCount::Count(count) => (count * (repeat.tracks.len() as u16)) as usize,
                RepetitionCount::AutoFill | RepetitionCount::AutoFit => repeat.tracks.len(),
            },
        })
        .sum()
}

/// Estimate definite size of a track sizing function
/// Simplified version of Taffy's track_definite_value
fn estimate_track_size(sizing_fn: taffy::TrackSizingFunction, parent_size: f32) -> f32 {
    // Try max sizing function first (prefer if definite)
    if let Some(max_val) = sizing_fn.max.definite_value(Some(parent_size), |_, _| 0.0) {
        // Floor by min if both definite
        if let Some(min_val) = sizing_fn.min.definite_value(Some(parent_size), |_, _| 0.0) {
            return max_val.max(min_val);
        }
        return max_val;
    }

    // Fall back to min sizing function
    if let Some(min_val) = sizing_fn.min.definite_value(Some(parent_size), |_, _| 0.0) {
        return min_val;
    }

    // Can't resolve - assume 100px placeholder
    100.0
}

/// Calculate total space used by tracks (excluding auto-repeat if exclude_auto = true)
fn calculate_track_space<'a, I>(
    tracks: I,
    _style: impl std::any::Any,
    container_size: f32,
    exclude_auto: bool,
) -> f32
where
    I: Iterator<Item = taffy::GenericGridTemplateComponent<String, &'a taffy::GridTemplateRepetition<String>>> + Clone,
{
    tracks
        .map(|track_def| match track_def {
            taffy::GenericGridTemplateComponent::Single(sizing_fn) => {
                estimate_track_size(sizing_fn, container_size)
            }
            taffy::GenericGridTemplateComponent::Repeat(repeat) => {
                match repeat.count {
                    RepetitionCount::Count(count) => {
                        let sum: f32 = repeat.tracks.iter()
                            .map(|sf| estimate_track_size(*sf, container_size))
                            .sum();
                        sum * (count as f32)
                    }
                    RepetitionCount::AutoFill | RepetitionCount::AutoFit => {
                        if exclude_auto { 0.0 } else {
                            repeat.tracks.iter()
                                .map(|sf| estimate_track_size(*sf, container_size))
                                .sum()
                        }
                    }
                }
            }
        })
        .sum()
}

/// Get track count from the definite (non-masonry) axis
/// Now supports auto-fill/auto-fit with dynamic calculation
#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout
pub fn get_definite_axis_track_count(
    tree: &BaseDocument,
    node_id: NodeId,
    masonry_axis: AbstractAxis,
    available_size: Option<f32>,
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

    // Get track template for grid axis
    let tracks = match masonry_axis {
        AbstractAxis::Block => style_wrapper.grid_template_columns(),
        AbstractAxis::Inline => style_wrapper.grid_template_rows(),
    };

    let Some(tracks) = tracks else {
        return Ok(1);
    };

    // Check if auto-repeat exists
    if has_auto_repeat_tracks(tracks.clone()).is_some() {
        // Use auto-repeat calculation
        return calculate_auto_repeat_track_count(tree, node_id, masonry_axis, available_size);
    }

    // No auto-repeat, count normally
    Ok(tracks.count().max(1))
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
