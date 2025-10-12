//! Track counting utilities for CSS Grid Level 3 masonry layout
//!
//! Handles counting of tracks in the definite (non-masonry) axis.

use taffy::{GridContainerStyle, ResolveOrZero};
use taffy::geometry::AbstractAxis;
use taffy::prelude::NodeId;
use taffy::RepetitionCount;

use super::super::grid_errors::GridPreprocessingError;
use crate::BaseDocument;

/// Result of track counting that includes auto-fit range information
#[derive(Debug, Clone)]
pub struct TrackCountResult {
    /// Total number of tracks
    pub count: usize,
    /// Range of auto-fit tracks (start_index, end_index) if auto-fit is used
    pub auto_fit_range: Option<(usize, usize)>,
}

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
) -> Result<TrackCountResult, GridPreprocessingError> {
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
        return Ok(TrackCountResult {
            count: 1,
            auto_fit_range: None,
        });
    };

    // Check for auto-repeat
    let auto_repeat_type = has_auto_repeat_tracks(tracks.clone());
    if auto_repeat_type.is_none() {
        // No auto-repeat, count normally
        return Ok(TrackCountResult {
            count: tracks.count(),
            auto_fit_range: None,
        });
    }

    // If no available size, default to 1 repetition
    let Some(container_size) = available_size else {
        let count = count_tracks_with_one_auto_repetition(tracks);
        return Ok(TrackCountResult {
            count,
            auto_fit_range: None,
        });
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
        let count = count_tracks_with_one_auto_repetition(tracks);
        return Ok(TrackCountResult {
            count,
            auto_fit_range: None,
        });
    };

    let repeat_track_count = auto_repeat.tracks.len() as u16;

    // Calculate space used by non-repeating tracks
    let non_repeating_space: f32 = calculate_track_space(tracks.clone(), container_size, true, tree, node_id, masonry_axis);

    // Calculate space per repetition
    let per_repetition_space: f32 = auto_repeat.tracks.iter()
        .map(|sizing_fn| estimate_track_size(*sizing_fn, container_size, tree, node_id, masonry_axis))
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

    // Calculate space used by non-auto tracks INCLUDING their internal gaps
    let non_auto_gaps = if non_auto_count > 0 {
        (non_auto_count.saturating_sub(1) as f32) * gap_size
    } else {
        0.0
    };
    let non_auto_total_space = non_repeating_space + non_auto_gaps;
    
    // Calculate space needed for first auto-fit track
    // Includes: gap before it (if there are non-auto tracks) + track size
    let first_auto_track_space = if non_auto_count > 0 {
        gap_size + per_repetition_space  // Gap before + track
    } else {
        per_repetition_space  // No gap if it's the first track
    };
    
    let first_repetition_size = non_auto_total_space + first_auto_track_space;
    
    if first_repetition_size > container_size {
        // First repetition doesn't fit, so return only non-auto tracks
        let total_tracks = non_auto_count as usize;
        // No auto-fit tracks were created, so no range to collapse
        return Ok(TrackCountResult {
            count: total_tracks,
            auto_fit_range: None,
        });
    }

    // Calculate additional repetitions
    // Each additional repetition adds:
    // - 1 gap before the repetition
    // - per_repetition_space (sum of track sizes in the repetition)
    // - (repeat_track_count - 1) gaps between tracks in the repetition
    // Total: gap_size + per_repetition_space + ((repeat_track_count - 1) * gap_size)
    // Simplified: per_repetition_space + (repeat_track_count * gap_size)
    let per_additional_rep = per_repetition_space + ((repeat_track_count as f32) * gap_size);
    
    let remaining_space = container_size - first_repetition_size;

    // Guard against division by zero when all tracks are intrinsic and very small
    const MIN_DIVISOR: f32 = 1.0;
    if per_additional_rep < MIN_DIVISOR {
        // If per-repetition size is essentially zero, use conservative upper bound
        // Based on minimum reasonable track size (1em ~= 16px)
        let min_track_size = if let Some(styles) = tree.node_from_id(node_id.into()).primary_styles() {
            styles.get_font().font_size.computed_size().px()
        } else {
            16.0
        };
        let max_reasonable_reps = ((container_size - first_repetition_size) / min_track_size).ceil() as u16;
        let total_repetitions = 1 + max_reasonable_reps;
        let total_tracks = (non_auto_count + (repeat_track_count * total_repetitions)) as usize;
        // Calculate auto-fit range if using auto-fit
        let auto_fit_range = if matches!(auto_repeat_type, Some(RepetitionCount::AutoFit)) {
            let tracks_before_auto = calculate_tracks_before_auto_repeat(tracks.clone());
            Some((tracks_before_auto, tracks_before_auto + (repeat_track_count * total_repetitions) as usize))
        } else {
            None
        };
        return Ok(TrackCountResult {
            count: total_tracks,
            auto_fit_range,
        });
    }

    let remaining_space = container_size - first_repetition_size;
    let additional_reps = (remaining_space / per_additional_rep).floor() as u16;
    let total_repetitions = additional_reps + 1;

    let total_tracks = (non_auto_count + (repeat_track_count * total_repetitions)) as usize;

    // Calculate auto-fit range if using auto-fit
    let auto_fit_range = if matches!(auto_repeat_type, Some(RepetitionCount::AutoFit)) {
        let tracks_before_auto = calculate_tracks_before_auto_repeat(tracks);
        Some((tracks_before_auto, tracks_before_auto + (repeat_track_count * total_repetitions) as usize))
    } else {
        None
    };

    Ok(TrackCountResult {
        count: total_tracks,
        auto_fit_range,
    })
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

/// Calculate number of tracks before the auto-repeat section
fn calculate_tracks_before_auto_repeat<'a, I>(tracks: I) -> usize
where
    I: Iterator<Item = taffy::GenericGridTemplateComponent<String, &'a taffy::GridTemplateRepetition<String>>>,
{
    let mut count = 0;
    for track_def in tracks {
        match track_def {
            taffy::GenericGridTemplateComponent::Single(_) => {
                count += 1;
            }
            taffy::GenericGridTemplateComponent::Repeat(repeat) => {
                match repeat.count {
                    RepetitionCount::Count(repeat_count) => {
                        count += (repeat_count * (repeat.tracks.len() as u16)) as usize;
                    }
                    RepetitionCount::AutoFill | RepetitionCount::AutoFit => {
                        // Found auto-repeat, return count of tracks before it
                        return count;
                    }
                }
            }
        }
    }
    count
}

/// Estimate track size with intrinsic sizing support
/// For intrinsic sizing functions, samples grid items to get realistic size estimates
fn estimate_track_size(
    sizing_fn: taffy::TrackSizingFunction,
    parent_size: f32,
    tree: &BaseDocument,
    node_id: NodeId,
    masonry_axis: AbstractAxis,
) -> f32 {
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

    // Intrinsic sizing - sample actual grid items for realistic estimate
    estimate_intrinsic_track_size(tree, node_id, masonry_axis, parent_size)
}

/// Estimate intrinsic track size using font-based heuristic
/// For intrinsic sizing functions (min-content, max-content, auto), we use a heuristic
/// based on the container's font size since we cannot measure items before track counting
fn estimate_intrinsic_track_size(
    tree: &BaseDocument,
    node_id: NodeId,
    _masonry_axis: AbstractAxis,
    _container_size: f32,
) -> f32 {
    let node = tree.node_from_id(node_id.into());
    
    // Use font-based heuristic: 3em is a reasonable estimate for intrinsic content
    // This matches common text line height (1.5em) + some vertical spacing
    if let Some(styles) = node.primary_styles() {
        let font_size = styles.get_font().font_size.computed_size().px();
        return font_size * 3.0;
    }

    // Final fallback: 48px (3 * 16px standard font)
    48.0
}

/// Calculate total space used by tracks (excluding auto-repeat if exclude_auto = true)
fn calculate_track_space<'a, I>(
    tracks: I,
    container_size: f32,
    exclude_auto: bool,
    tree: &BaseDocument,
    node_id: NodeId,
    masonry_axis: AbstractAxis,
) -> f32
where
    I: Iterator<Item = taffy::GenericGridTemplateComponent<String, &'a taffy::GridTemplateRepetition<String>>> + Clone,
{
    tracks
        .map(|track_def| match track_def {
            taffy::GenericGridTemplateComponent::Single(sizing_fn) => {
                estimate_track_size(sizing_fn, container_size, tree, node_id, masonry_axis)
            }
            taffy::GenericGridTemplateComponent::Repeat(repeat) => {
                match repeat.count {
                    RepetitionCount::Count(count) => {
                        let sum: f32 = repeat.tracks.iter()
                            .map(|sf| estimate_track_size(*sf, container_size, tree, node_id, masonry_axis))
                            .sum();
                        sum * (count as f32)
                    }
                    RepetitionCount::AutoFill | RepetitionCount::AutoFit => {
                        if exclude_auto { 0.0 } else {
                            repeat.tracks.iter()
                                .map(|sf| estimate_track_size(*sf, container_size, tree, node_id, masonry_axis))
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
        // Use auto-repeat calculation and extract count
        let result = calculate_auto_repeat_track_count(tree, node_id, masonry_axis, available_size)?;
        return Ok(result.count);
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

/// Check if grid template uses auto-fit (not auto-fill)
/// Returns true if any repeat definition uses RepetitionCount::AutoFit
pub fn check_if_uses_auto_fit(
    tree: &BaseDocument,
    node_id: NodeId,
    masonry_axis: AbstractAxis,
) -> Result<bool, GridPreprocessingError> {
    let node = tree.node_from_id(node_id.into());
    let computed_styles = node.primary_styles().ok_or_else(|| {
        GridPreprocessingError::preprocessing_failed(
            "auto_fit_check",
            node_id.into(),
            "Primary styles not available",
        )
    })?;

    let style_wrapper = stylo_taffy::TaffyStyloStyle::from(computed_styles);

    // Get tracks for grid axis (opposite of masonry axis)
    let tracks = match masonry_axis {
        AbstractAxis::Block => style_wrapper.grid_template_columns(),
        AbstractAxis::Inline => style_wrapper.grid_template_rows(),
    };

    let Some(tracks) = tracks else {
        return Ok(false);
    };

    // Check for auto-fit in any repeat definition
    Ok(tracks
        .filter_map(|component| match component {
            taffy::GenericGridTemplateComponent::Repeat(repeat) => Some(repeat.count),
            _ => None,
        })
        .any(|count| matches!(count, RepetitionCount::AutoFit)))
}
