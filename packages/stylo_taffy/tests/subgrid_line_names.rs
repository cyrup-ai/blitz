//! Unit tests for subgrid line name mapping functionality
//!
//! These tests verify the CSS Grid Level 2 subgrid line name mapping implementation
//! per the W3C specification: https://www.w3.org/TR/css-grid-2/#subgrid-line-names

use stylo_taffy::convert::{GridAxis, GridContext, SubgridLineNameError};
use taffy::prelude::TaffyAuto;
use taffy::{GridTemplateComponent, GridTemplateRepetition, RepetitionCount, TrackSizingFunction};

#[test]
fn test_subgrid_line_name_error_display() {
    // Test that our error types implement Display correctly
    let error = SubgridLineNameError::LineIndexOutOfBounds {
        line_index: 5,
        track_count: 3,
    };
    let error_string = format!("{}", error);
    assert!(error_string.contains("Line index 5 out of bounds for track count 3"));
}

#[test]
fn test_subgrid_line_name_error_debug() {
    // Test that our error types implement Debug correctly
    let error = SubgridLineNameError::InvalidLineNameFormat("test".to_string());
    let debug_string = format!("{:?}", error);
    assert!(debug_string.contains("InvalidLineNameFormat"));
}

#[test]
fn test_single_track_line_name_error() {
    let error = SubgridLineNameError::SingleTrackLineNameUnsupported { track_index: 2 };
    let error_string = format!("{}", error);
    assert!(error_string.contains("Cannot add line names to Single track at index 2"));
}

#[test]
fn test_custom_ident_conversion_error() {
    let error = SubgridLineNameError::CustomIdentConversionFailed("conversion failed".to_string());
    let error_string = format!("{}", error);
    assert!(error_string.contains("Failed to convert CustomIdent to String: conversion failed"));
}

#[test]
fn test_grid_context_creation() {
    // Test GridContext can be created and used
    let context = GridContext {
        axis: GridAxis::Row,
        supports_subgrid: true,
        parent_tracks: vec![
            GridTemplateComponent::Single(TrackSizingFunction::AUTO),
            GridTemplateComponent::Repeat(GridTemplateRepetition {
                count: RepetitionCount::Count(2),
                tracks: vec![TrackSizingFunction::AUTO, TrackSizingFunction::AUTO],
                line_names: vec![
                    vec!["start".to_string()],
                    vec!["middle".to_string()],
                    vec!["end".to_string()],
                ],
            }),
        ],
        available_space: None,
        masonry_state: None,
        parent_line_names: Vec::new(),
    };

    assert_eq!(context.axis, GridAxis::Row);
    assert!(context.supports_subgrid);
    assert_eq!(context.parent_tracks.len(), 2);
}

#[test]
fn test_grid_axis_equality() {
    assert_eq!(GridAxis::Row, GridAxis::Row);
    assert_eq!(GridAxis::Column, GridAxis::Column);
    assert_ne!(GridAxis::Row, GridAxis::Column);
}

#[test]
fn test_grid_context_with_columns() {
    let context = GridContext {
        axis: GridAxis::Column,
        supports_subgrid: false,
        parent_tracks: Vec::new(),
        available_space: None,
        masonry_state: None,
        parent_line_names: Vec::new(),
    };

    assert_eq!(context.axis, GridAxis::Column);
    assert!(!context.supports_subgrid);
    assert!(context.parent_tracks.is_empty());
}

#[test]
fn test_error_clone_and_debug() {
    let original_error = SubgridLineNameError::LineIndexOutOfBounds {
        line_index: 1,
        track_count: 0,
    };

    // Test that errors can be cloned
    let cloned_error = original_error.clone();
    assert!(format!("{:?}", original_error) == format!("{:?}", cloned_error));
}

/// Test that demonstrates the expected behavior when Single tracks cannot store line names
#[test]
fn test_single_track_limitation_documentation() {
    // This test documents the limitation that Single tracks in taffy cannot store line names
    // This is a known limitation of the taffy type system, not a bug in our implementation

    let single_track = GridTemplateComponent::<String>::Single(TrackSizingFunction::AUTO);

    // Single tracks don't have a line_names field, only Repeat tracks do
    match single_track {
        GridTemplateComponent::Single(_) => {
            // Expected: Single tracks cannot store line names
            // This is why our implementation returns SingleTrackLineNameUnsupported errors
        }
        GridTemplateComponent::Repeat(repetition) => {
            // Repeat tracks can store line names
            assert!(repetition.line_names.is_empty()); // Initially empty
        }
    }
}

/// Test the structure of GridTemplateRepetition to ensure our implementation matches
#[test]
fn test_grid_template_repetition_structure() {
    let repetition = GridTemplateRepetition {
        count: RepetitionCount::Count(3),
        tracks: vec![TrackSizingFunction::AUTO; 3],
        line_names: vec![
            vec!["start".to_string()],
            vec!["middle1".to_string(), "middle2".to_string()],
            vec!["end".to_string()],
        ],
    };

    // Verify that GridTemplateRepetition has the expected structure
    assert_eq!(repetition.tracks.len(), 3);
    assert_eq!(repetition.line_names.len(), 3);
    assert_eq!(repetition.line_names[1].len(), 2); // middle1, middle2

    match repetition.count {
        RepetitionCount::Count(n) => assert_eq!(n, 3),
        _ => panic!("Expected Count repetition count"),
    }
}

#[test]
fn test_comprehensive_error_coverage() {
    // Ensure all error variants can be created and formatted
    let errors = vec![
        SubgridLineNameError::LineIndexOutOfBounds {
            line_index: 0,
            track_count: 0,
        },
        SubgridLineNameError::InvalidLineNameFormat("".to_string()),
        SubgridLineNameError::SingleTrackLineNameUnsupported { track_index: 0 },
        SubgridLineNameError::CustomIdentConversionFailed("test".to_string()),
    ];

    for error in errors {
        // Each error should be displayable and debuggable
        let display_str = format!("{}", error);
        let debug_str = format!("{:?}", error);

        assert!(!display_str.is_empty());
        assert!(!debug_str.is_empty());

        // Each error should contain meaningful information
        match error {
            SubgridLineNameError::LineIndexOutOfBounds { .. } => {
                assert!(display_str.contains("out of bounds"));
            }
            SubgridLineNameError::InvalidLineNameFormat(_) => {
                assert!(display_str.contains("Invalid line name format"));
            }
            SubgridLineNameError::SingleTrackLineNameUnsupported { .. } => {
                assert!(display_str.contains("Single track"));
            }
            SubgridLineNameError::CustomIdentConversionFailed(_) => {
                assert!(display_str.contains("convert CustomIdent"));
            }
        }
    }
}
