//! Type definitions for CSS Grid Level 2 subgrid preprocessing
//!
//! This module contains all the data structures and types needed for subgrid
//! track inheritance, coordinate transformation, and nested subgrid coordination.

use std::collections::HashMap;

use taffy::prelude::*;

/// Inherited track information for subgrids
/// This stores the track definitions and line names inherited from parent grids
/// without modifying computed styles (which is not how browser style systems work)
#[derive(Debug, Clone)]
pub struct SubgridTrackInheritance {
    /// Row tracks inherited from parent grid
    pub inherited_row_tracks: Vec<taffy::TrackSizingFunction>,
    /// Column tracks inherited from parent grid
    pub inherited_column_tracks: Vec<taffy::TrackSizingFunction>,
    /// Row line names inherited from parent grid
    pub inherited_row_line_names: Vec<Vec<String>>,
    /// Column line names inherited from parent grid
    pub inherited_column_line_names: Vec<Vec<String>>,
    /// Whether this subgrid uses inherited row tracks
    pub uses_subgrid_rows: bool,
    /// Whether this subgrid uses inherited column tracks
    pub uses_subgrid_columns: bool,
    /// Coordinate transformation for mapping subgrid to parent coordinates
    pub coordinate_transform: CoordinateTransform,
}

impl Default for SubgridTrackInheritance {
    fn default() -> Self {
        Self {
            inherited_row_tracks: Vec::new(),
            inherited_column_tracks: Vec::new(),
            inherited_row_line_names: Vec::new(),
            inherited_column_line_names: Vec::new(),
            uses_subgrid_rows: false,
            uses_subgrid_columns: false,
            coordinate_transform: CoordinateTransform {
                row_offset: 0,
                column_offset: 0,
                row_scale: 1.0,
                column_scale: 1.0,
            },
        }
    }
}

/// Global storage for subgrid track inheritance data
/// This allows layout algorithms to access inherited track information
/// without modifying computed styles
pub type SubgridInheritanceRegistry = HashMap<NodeId, SubgridTrackInheritance>;

/// Coordinate transformation matrix for mapping between subgrid levels
#[derive(Debug, Clone)]
pub struct CoordinateTransform {
    /// Row coordinate offset in parent grid
    pub row_offset: usize,
    /// Column coordinate offset in parent grid
    pub column_offset: usize,
    /// Row scaling factor (usually 1:1 for subgrids)
    pub row_scale: f32,
    /// Column scaling factor (usually 1:1 for subgrids)
    pub column_scale: f32,
}

/// Track inheritance information for one level in the chain
#[derive(Debug, Clone)]
pub struct TrackInheritanceLevel {
    /// Subgrid node at this level
    pub subgrid_id: NodeId,
    /// Parent subgrid ID (None for root level)
    pub parent_subgrid_id: Option<NodeId>,
    /// Span within parent grid for row axis
    pub row_span_in_parent: Option<SubgridSpan>,
    /// Span within parent grid for column axis
    pub column_span_in_parent: Option<SubgridSpan>,
    /// Track mapping transformation for this level
    pub coordinate_transform: CoordinateTransform,
}

/// Effective tracks after inheritance processing
#[derive(Debug, Clone)]
pub struct EffectiveSubgridTracks {
    /// Final row tracks for this subgrid
    pub effective_row_tracks: Vec<taffy::TrackSizingFunction>,
    /// Final column tracks for this subgrid
    pub effective_column_tracks: Vec<taffy::TrackSizingFunction>,
    /// Final row line names
    pub effective_row_line_names: Vec<Vec<String>>,
    /// Final column line names
    pub effective_column_line_names: Vec<Vec<String>>,
}

/// Child subgrid span within parent subgrid coordinates
#[derive(Debug, Clone)]
pub struct ChildSubgridSpan {
    pub row_start: usize,
    pub row_end: usize,
    pub column_start: usize,
    pub column_end: usize,
}

/// Inherited line names from parent levels
#[derive(Debug, Clone)]
pub struct InheritedLineNames {
    pub row_names: Vec<Vec<String>>,
    pub column_names: Vec<Vec<String>>,
}

/// Line name mapping through inheritance chain
#[derive(Debug, Clone)]
pub struct LineNameMapping {
    /// Source line names
    pub source_names: Vec<String>,
    /// Target line names
    pub target_names: Vec<String>,
    /// Mapping level in inheritance chain
    pub level: usize,
}

/// Grid position span for subgrid placement
#[derive(Debug, Clone)]
pub struct SubgridSpan {
    /// Start line (1-based indexing)
    pub start: usize,
    /// End line (1-based indexing)
    pub end: usize,
}

/// Track sizing contribution for subgrid items
#[derive(Debug, Clone)]
pub struct TrackSizingContribution {
    /// Contributing item ID
    pub item_id: NodeId,
    /// Track index in parent grid
    pub track_index: usize,
    /// Contribution type (row or column)
    pub axis: GridAxis,
    /// Minimum size contribution
    pub min_size: f32,
    /// Maximum size contribution
    pub max_size: f32,
    /// Preferred size contribution
    pub preferred_size: Option<f32>,
}

/// Grid axis enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridAxis {
    Row,
    Column,
}

/// Subgrid item placement result
#[derive(Debug, Clone)]
pub struct ItemPlacement {
    /// Item node ID
    pub item_id: NodeId,
    /// Row start position in parent grid coordinates
    pub parent_grid_row_start: usize,
    /// Row end position in parent grid coordinates
    pub parent_grid_row_end: usize,
    /// Column start position in parent grid coordinates
    pub parent_grid_column_start: usize,
    /// Column end position in parent grid coordinates
    pub parent_grid_column_end: usize,
}

/// Subgrid layout processing result
#[derive(Debug, Clone)]
pub struct SubgridLayoutResult {
    /// Final item placements
    pub item_placements: Vec<ItemPlacement>,
    /// Size contributions to parent grid
    pub size_contributions: Vec<TrackSizingContribution>,
    /// Whether layout was successful
    pub success: bool,
    /// Warning messages
    pub warnings: Vec<String>,
}
