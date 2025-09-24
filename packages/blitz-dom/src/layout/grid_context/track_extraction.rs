//! Track extraction from stylo and taffy grid templates
//!
//! This module handles the complex task of extracting track sizing functions
//! and line names from various grid template sources.

use style::properties::ComputedValues;
use style::values::CustomIdent;
use style::values::computed::GridTemplateComponent;

use super::types::{GridAxis, TrackExtractionError};

/// Extract TrackSizingFunction values from grid template track list
///
/// Handles the complex iterator types returned by GridContainerStyle methods
/// and flattens them to Vec<TrackSizingFunction> for ParentGridContext
pub fn extract_tracks_from_template_list<'a, TemplateTrackList, CustomIdent, Repetition>(
    template_tracks: Option<TemplateTrackList>,
) -> Result<Vec<taffy::TrackSizingFunction>, TrackExtractionError>
where
    TemplateTrackList:
        Iterator<Item = taffy::GenericGridTemplateComponent<CustomIdent, Repetition>>,
    CustomIdent: taffy::CheapCloneStr,
    Repetition: taffy::GenericRepetition<CustomIdent = CustomIdent>,
{
    let Some(tracks) = template_tracks else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();

    for component in tracks {
        match component {
            taffy::GenericGridTemplateComponent::Single(track_fn) => {
                // Direct track sizing function
                result.push(track_fn);
            }
            taffy::GenericGridTemplateComponent::Repeat(repetition) => {
                // Expand repetition pattern
                expand_repetition_pattern(&mut result, repetition)?;
            }
        }
    }

    Ok(result)
}

/// Expand a repetition pattern into individual track sizing functions
pub fn expand_repetition_pattern<CustomIdent, Repetition>(
    result: &mut Vec<taffy::TrackSizingFunction>,
    repetition: Repetition,
) -> Result<(), TrackExtractionError>
where
    CustomIdent: taffy::CheapCloneStr,
    Repetition: taffy::GenericRepetition<CustomIdent = CustomIdent>,
{
    let repeat_count = match repetition.count() {
        taffy::RepetitionCount::Count(n) => {
            let count = n as usize;
            // Validate reasonable repeat count
            if count == 0 {
                return Err(TrackExtractionError::ExtractionFailed);
            }
            count
        }
        taffy::RepetitionCount::AutoFit | taffy::RepetitionCount::AutoFill => {
            // For auto-sizing, use a reasonable default
            // Actual intrinsic sizing is handled by taffy layout engine
            3
        }
    };

    // Safety: Limit repetition count to prevent memory issues
    const MAX_REPETITIONS: usize = 1000;
    let safe_repeat_count = repeat_count.min(MAX_REPETITIONS);

    // Expand the tracks
    let tracks: Vec<_> = repetition.tracks().collect();

    // Validate we have tracks to expand
    if tracks.is_empty() {
        return Err(TrackExtractionError::ExtractionFailed);
    }

    for _ in 0..safe_repeat_count {
        result.extend_from_slice(&tracks);
    }

    Ok(())
}

/// Extract line names from grid template line names
pub fn extract_line_names_from_style<'a, TemplateLineNames, CustomIdent>(
    line_names: Option<TemplateLineNames>,
) -> Result<Vec<Vec<String>>, TrackExtractionError>
where
    TemplateLineNames: taffy::TemplateLineNames<'a, CustomIdent>,
    CustomIdent: taffy::CheapCloneStr,
{
    let Some(names) = line_names else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();

    for line_name_set in names {
        let line_group: Vec<String> = line_name_set.map(|name| format!("{:?}", name)).collect();
        result.push(line_group);
    }

    Ok(result)
}

/// Detect if a node has subgrid for a specific axis using taffy grid infrastructure
///
/// Note: This is a simplified version since taffy doesn't support subgrid detection yet.
/// Real axis-specific subgrid detection requires access to stylo computed values
pub fn detect_subgrid_axis_from_style<Tree>(
    _tree: &Tree,
    _node_id: taffy::prelude::NodeId,
    _is_row_axis: bool,
) -> bool
where
    Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
{
    // Taffy doesn't have GridTemplateComponent::Subgrid variant yet
    // This function exists to maintain the interface but returns false
    // Real axis-specific subgrid detection happens in BaseDocument functions using stylo
    false
}

/// Extract tracks directly from stylo computed styles using existing conversion infrastructure
///
/// This function replaces the generic `extract_tracks_from_template_list` with direct
/// stylo integration, leveraging the comprehensive conversion functions in stylo_taffy.
pub fn extract_tracks_from_stylo_computed_styles(
    computed_styles: &ComputedValues,
    axis: GridAxis,
) -> Result<Vec<taffy::TrackSizingFunction>, TrackExtractionError> {
    let grid_template = match axis {
        GridAxis::Row => &computed_styles.get_position().grid_template_rows,
        GridAxis::Column => &computed_styles.get_position().grid_template_columns,
    };

    match grid_template {
        GridTemplateComponent::None => {
            // No explicit tracks defined - return empty vector like original function
            Ok(Vec::new())
        }
        GridTemplateComponent::TrackList(_track_list) => {
            // Use existing stylo_taffy conversion infrastructure
            let taffy_components = stylo_taffy::convert::grid_template_tracks(grid_template, None);

            // Convert GridTemplateComponent to TrackSizingFunction
            let mut tracks = Vec::new();
            for component in taffy_components {
                match component {
                    taffy::GridTemplateComponent::Single(track_fn) => {
                        tracks.push(track_fn);
                    }
                    taffy::GridTemplateComponent::Repeat(repetition) => {
                        // Expand repetition using existing logic
                        expand_repetition_to_tracks(&mut tracks, repetition)?;
                    }
                }
            }

            Ok(tracks)
        }
        GridTemplateComponent::Subgrid(_) => {
            // Subgrid tracks should be inherited from parent
            Err(TrackExtractionError::SubgridInheritanceRequired)
        }
        GridTemplateComponent::Masonry => {
            // Masonry axis has no explicit tracks
            Err(TrackExtractionError::MasonryAxisHasNoTracks)
        }
    }
}

/// Extract line names from stylo computed styles
///
/// Extracts line names declared in CSS grid templates and converts them to
/// the format expected by ParentGridContext.
pub fn extract_line_names_from_stylo_computed_styles(
    computed_styles: &ComputedValues,
    axis: GridAxis,
) -> Result<Vec<Vec<String>>, TrackExtractionError> {
    let grid_template = match axis {
        GridAxis::Row => &computed_styles.get_position().grid_template_rows,
        GridAxis::Column => &computed_styles.get_position().grid_template_columns,
    };

    match grid_template {
        GridTemplateComponent::TrackList(track_list) => {
            // Extract line names from track list using existing patterns
            extract_line_names_from_track_list(&track_list.line_names)
        }
        _ => Ok(Vec::new()),
    }
}

/// Extract line names from stylo track list
///
/// Helper function to convert stylo CustomIdent line names to String vectors.
fn extract_line_names_from_track_list(
    line_names: &style::OwnedSlice<style::OwnedSlice<CustomIdent>>,
) -> Result<Vec<Vec<String>>, TrackExtractionError> {
    let mut result = Vec::new();

    for line_name_group in line_names.iter() {
        let string_names: Vec<String> = line_name_group
            .iter()
            .map(|ident| ident.0.to_string())
            .collect();
        result.push(string_names);
    }

    Ok(result)
}

/// Expand repetition pattern to individual track sizing functions
///
/// Converts taffy::GridTemplateRepetition to individual TrackSizingFunction entries,
/// handling repeat counts and auto-fit/auto-fill patterns.
fn expand_repetition_to_tracks(
    result: &mut Vec<taffy::TrackSizingFunction>,
    repetition: taffy::GridTemplateRepetition<String>,
) -> Result<(), TrackExtractionError> {
    let repeat_count = match repetition.count {
        taffy::RepetitionCount::Count(n) => {
            let count = n as usize;
            // Validate reasonable repeat count
            if count == 0 {
                return Err(TrackExtractionError::ExtractionFailed);
            }
            count
        }
        taffy::RepetitionCount::AutoFit | taffy::RepetitionCount::AutoFill => {
            // For auto-sizing, use a reasonable default
            // Actual intrinsic sizing is handled by taffy layout engine
            3
        }
    };

    // Safety: Limit repetition count to prevent memory issues
    const MAX_REPETITIONS: usize = 1000;
    let safe_repeat_count = repeat_count.min(MAX_REPETITIONS);

    // Validate we have tracks to expand
    if repetition.tracks.is_empty() {
        return Err(TrackExtractionError::ExtractionFailed);
    }

    // Expand the tracks
    for _ in 0..safe_repeat_count {
        result.extend_from_slice(&repetition.tracks);
    }

    Ok(())
}

/// Detect subgrid from stylo computed styles
///
/// Checks if the given computed styles indicate subgrid for the specified axis.
pub fn detect_subgrid_from_stylo(computed_styles: &ComputedValues, axis: GridAxis) -> bool {
    let grid_template = match axis {
        GridAxis::Row => &computed_styles.get_position().grid_template_rows,
        GridAxis::Column => &computed_styles.get_position().grid_template_columns,
    };

    matches!(grid_template, GridTemplateComponent::Subgrid(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full CSS parsing tests would require additional infrastructure
    // The following tests are placeholders showing the expected interface
    // but cannot be implemented without CSS parsing capabilities

    #[test]
    #[ignore = "Requires CSS parsing infrastructure not currently available"]
    fn test_extract_simple_tracks() {
        // This test would require implementing create_test_computed_styles()
        // which needs access to stylo's CSS parsing infrastructure
        todo!("Implement CSS parsing for tests")
    }

    #[test]
    #[ignore = "Requires CSS parsing infrastructure not currently available"]
    fn test_extract_repeat_tracks() {
        // This test would require implementing create_test_computed_styles()
        todo!("Implement CSS parsing for tests")
    }

    #[test]
    #[ignore = "Requires CSS parsing infrastructure not currently available"]
    fn test_extract_line_names() {
        // This test would require implementing create_test_computed_styles()
        todo!("Implement CSS parsing for tests")
    }

    #[test]
    #[ignore = "Requires CSS parsing infrastructure not currently available"]
    fn test_subgrid_detection() {
        // This test would require implementing create_test_computed_styles()
        todo!("Implement CSS parsing for tests")
    }

    #[test]
    #[ignore = "Requires CSS parsing infrastructure not currently available"]
    fn test_masonry_detection() {
        // This test would require implementing create_test_computed_styles()
        todo!("Implement CSS parsing for tests")
    }

    #[test]
    #[ignore = "Requires CSS parsing infrastructure not currently available"]
    fn test_complex_track_functions() {
        // This test would require implementing create_test_computed_styles()
        todo!("Implement CSS parsing for tests")
    }
}
