//! Error handling for CSS Grid Level 2 and 3 features
//!
//! This module provides comprehensive error types for subgrid, masonry,
//! and grid preprocessing operations with detailed error messages and
//! graceful degradation support.

use thiserror::Error;

/// Enhanced SubgridError with detailed error messages and context
#[derive(Error, Debug, Clone)]
#[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid implementation
pub enum SubgridError {
    #[error("No parent grid container found for subgrid at node {node_id}")]
    NoParentGrid { node_id: usize },

    #[error("Style access failed for node {node_id}: {reason}")]
    StyleAccess { node_id: usize, reason: String },

    #[error("Invalid track inheritance: cannot inherit {track_type} tracks from parent grid")]
    InvalidTrackInheritance { track_type: String },

    #[error("Line name mapping failed: {source_line} -> {target_line} mapping error: {reason}")]
    LineNameMappingFailed {
        source_line: String,
        target_line: String,
        reason: String,
    },

    #[error("Subgrid not supported: {reason} (fallback to standard grid)")]
    SubgridNotSupported { reason: String },

    #[error("Track count mismatch: expected {expected} tracks, found {actual} in parent grid")]
    TrackCountMismatch { expected: usize, actual: usize },

    #[error("Nested subgrid depth {depth} exceeds maximum allowed depth {max_depth}")]
    ExcessiveNestingDepth { depth: usize, max_depth: usize },

    #[error("Parent grid context validation failed: {reason}")]
    ParentContextValidationFailed { reason: String },

    #[error("Subgrid coordinate mapping failed: {details}")]
    CoordinateMappingFailed { details: String },

    #[error("Subgrid coordination failed: {details}")]
    CoordinationFailed { details: String },
}

/// Enhanced MasonryError with detailed error messages and context
#[derive(Error, Debug, Clone)]
#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout implementation
pub enum MasonryError {
    #[error("Invalid masonry track count {track_count}: must be between {min} and {max}")]
    InvalidTrackCount {
        track_count: usize,
        min: usize,
        max: usize,
    },

    #[error("Masonry item placement failed at track {track_index}: {reason}")]
    PlacementFailed { track_index: usize, reason: String },

    #[error("Content sizing failed for masonry item {item_node_id}: {reason}")]
    ContentSizingFailed { item_node_id: usize, reason: String },

    #[error("Grid item collection failed: {reason}")]
    ItemCollectionFailed { reason: String },

    #[error("Masonry track span {span} exceeds available tracks {available_tracks}")]
    TrackSpanExceedsAvailable {
        span: usize,
        available_tracks: usize,
    },

    #[error("Auto-placement cursor overflow: cursor {cursor} exceeds track count {track_count}")]
    AutoPlacementCursorOverflow { cursor: usize, track_count: usize },

    #[error("Masonry axis {axis:?} configuration invalid: {reason}")]
    InvalidAxisConfiguration {
        axis: taffy::AbsoluteAxis,
        reason: String,
    },
}

/// Unified error hierarchy for all grid preprocessing operations
///
/// Provides comprehensive error handling across subgrid, masonry, and track extraction
/// operations with detailed error messages and graceful degradation support.
#[derive(Error, Debug, Clone)]
#[allow(dead_code)] // Infrastructure for grid preprocessing operations
pub enum GridPreprocessingError {
    /// Subgrid-related errors with nested SubgridError details
    #[error("Subgrid preprocessing failed: {0}")]
    Subgrid(#[from] SubgridError),

    /// Masonry-related errors with nested MasonryError details
    #[error("Masonry preprocessing failed: {0}")]
    Masonry(#[from] MasonryError),

    /// Track extraction and conversion errors
    #[error("Grid track extraction failed: {reason}")]
    TrackExtractionFailed { reason: String },

    /// Parent grid context resolution errors
    #[error("Parent grid context resolution failed for node {node_id}: {reason}")]
    ParentContextResolutionFailed { node_id: usize, reason: String },

    /// Generic grid preprocessing failures
    #[error("Grid preprocessing failed: {operation} at node {node_id} - {details}")]
    PreprocessingFailed {
        operation: String,
        node_id: usize,
        details: String,
    },
}

/// Result type aliases for consistent error handling
#[allow(dead_code)] // Used by grid layout infrastructure
pub type GridResult<T> = Result<T, GridPreprocessingError>;
pub type SubgridResult<T> = Result<T, SubgridError>;
#[allow(dead_code)] // Used by masonry layout infrastructure
pub type MasonryResult<T> = Result<T, MasonryError>;

/// Helper functions for creating contextual errors
#[allow(dead_code)] // Infrastructure for grid preprocessing
impl GridPreprocessingError {
    /// Create track extraction error with context
    pub fn track_extraction_failed(reason: impl Into<String>) -> Self {
        Self::TrackExtractionFailed {
            reason: reason.into(),
        }
    }

    /// Create parent context resolution error with context
    pub fn parent_context_failed(node_id: usize, reason: impl Into<String>) -> Self {
        Self::ParentContextResolutionFailed {
            node_id,
            reason: reason.into(),
        }
    }

    /// Create generic preprocessing error with full context
    pub fn preprocessing_failed(
        operation: impl Into<String>,
        node_id: usize,
        details: impl Into<String>,
    ) -> Self {
        Self::PreprocessingFailed {
            operation: operation.into(),
            node_id,
            details: details.into(),
        }
    }
}

#[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid
impl SubgridError {
    /// Create subgrid not supported error with reason
    pub fn not_supported(reason: impl Into<String>) -> Self {
        Self::SubgridNotSupported {
            reason: reason.into(),
        }
    }

    /// Create line name mapping error with full context
    pub fn line_mapping_failed(
        source: impl Into<String>,
        target: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::LineNameMappingFailed {
            source_line: source.into(),
            target_line: target.into(),
            reason: reason.into(),
        }
    }

    /// Create CSS identifier validation error
    pub fn invalid_css_identifier(
        name: impl Into<String>,
        line_index: usize,
        reason: impl Into<String>,
    ) -> Self {
        Self::LineNameMappingFailed {
            source_line: name.into(),
            target_line: format!("line {}", line_index + 1),
            reason: reason.into(),
        }
    }

    /// Create line name span validation error  
    pub fn invalid_line_name_span(
        span_start: usize,
        span_end: usize,
        parent_line_count: usize,
    ) -> Self {
        Self::line_mapping_failed(
            format!("span {}..{}", span_start, span_end),
            format!("parent lines [0..{}]", parent_line_count),
            "Subgrid span exceeds parent grid line count",
        )
    }
}

#[allow(dead_code)] // Infrastructure for CSS Grid Level 3 masonry layout
impl MasonryError {
    /// Create placement failed error with context
    pub fn placement_failed(track_index: usize, reason: impl Into<String>) -> Self {
        Self::PlacementFailed {
            track_index,
            reason: reason.into(),
        }
    }

    /// Create invalid axis configuration error
    pub fn invalid_axis(axis: taffy::AbsoluteAxis, reason: impl Into<String>) -> Self {
        Self::InvalidAxisConfiguration {
            axis,
            reason: reason.into(),
        }
    }
}

impl From<crate::layout::grid_context::TrackExtractionError> for GridPreprocessingError {
    fn from(err: crate::layout::grid_context::TrackExtractionError) -> Self {
        Self::preprocessing_failed(
            "track_extraction",
            0,
            format!("Track extraction failed: {:?}", err),
        )
    }
}
