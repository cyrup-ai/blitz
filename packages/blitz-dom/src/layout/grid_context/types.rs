//! Type definitions for grid context resolution
//!
//! This module provides core types used throughout the grid context system.

use taffy::prelude::NodeId;

/// Parent grid context for subgrid track inheritance
#[derive(Debug, Clone, Default)]
pub struct ParentGridContext {
    /// Resolved parent grid tracks for row inheritance
    pub parent_row_tracks: Vec<taffy::TrackSizingFunction>,
    /// Resolved parent grid tracks for column inheritance
    pub parent_column_tracks: Vec<taffy::TrackSizingFunction>,
    /// Parent grid line names for row axis
    pub parent_row_line_names: Vec<Vec<String>>,
    /// Parent grid line names for column axis
    pub parent_column_line_names: Vec<Vec<String>>,
    /// Whether parent has subgrid in rows
    pub parent_has_subgrid_rows: bool,
    /// Whether parent has subgrid in columns
    pub parent_has_subgrid_columns: bool,
    /// Number of row tracks in parent grid
    pub row_track_count: usize,
    /// Number of column tracks in parent grid
    pub column_track_count: usize,
}

/// Error types for generic tree grid context resolution
#[derive(Debug, Clone)]
pub enum GridContextError {
    /// Track extraction failed due to complex types
    TrackExtractionFailed,
}

/// Error types for track extraction operations
#[derive(Debug, Clone)]
pub enum TrackExtractionError {
    /// Generic extraction failed
    ExtractionFailed,
    /// Subgrid inheritance required (not an error, but needs special handling)
    SubgridInheritanceRequired,
    /// Masonry axis has no explicit tracks
    MasonryAxisHasNoTracks,
    /// Unsupported calc expression
    UnsupportedCalcExpression(String),
    /// Invalid track size
    InvalidTrackSize(String),
}

impl std::fmt::Display for TrackExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackExtractionError::ExtractionFailed => write!(f, "Track extraction failed"),
            TrackExtractionError::SubgridInheritanceRequired => {
                write!(f, "Subgrid inheritance required")
            }
            TrackExtractionError::MasonryAxisHasNoTracks => {
                write!(f, "Masonry axis has no explicit tracks")
            }
            TrackExtractionError::UnsupportedCalcExpression(expr) => {
                write!(f, "Unsupported calc expression: {}", expr)
            }
            TrackExtractionError::InvalidTrackSize(size) => {
                write!(f, "Invalid track size: {}", size)
            }
        }
    }
}

/// Grid axis enumeration for track extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GridAxis {
    Row,
    Column,
}

/// Grid span for line name inheritance
///
/// Compatible with existing SubgridSpan in subgrid_preprocessing.rs
#[derive(Debug, Clone, Copy)]
pub struct GridSpan {
    pub start: usize,
    pub end: usize,
}

/// Subgrid inheritance level for nested line name resolution
///
/// Links to existing SubgridInheritanceLevel in subgrid_preprocessing.rs
#[derive(Debug, Clone)]
pub struct SubgridInheritanceLevel {
    pub parent_line_names: Vec<Vec<String>>,
    pub declared_names: Vec<String>,
    pub span_in_parent: GridSpan,
    pub subgrid_node_id: NodeId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_handling() {
        // Test comprehensive error handling
        let error_cases = vec![
            TrackExtractionError::SubgridInheritanceRequired,
            TrackExtractionError::MasonryAxisHasNoTracks,
            TrackExtractionError::UnsupportedCalcExpression("test".to_string()),
            TrackExtractionError::InvalidTrackSize("test".to_string()),
        ];

        for error in error_cases {
            // Each error should be displayable and debuggable
            let display_str = format!("{}", error);
            let debug_str = format!("{:?}", error);

            assert!(!display_str.is_empty());
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_grid_axis_equality() {
        assert_eq!(GridAxis::Row, GridAxis::Row);
        assert_eq!(GridAxis::Column, GridAxis::Column);
        assert_ne!(GridAxis::Row, GridAxis::Column);
    }
}
