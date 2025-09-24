//! Track-related data structures for CSS Grid Layout

use super::placement_types::ItemPlacement;
use super::types::TrackSizeContribution;

/// Track definition from parent grid
#[derive(Debug, Clone)]
pub struct TrackDefinition {
    /// Track size
    pub size: f32,

    /// Minimum size constraint
    pub min_size: f32,

    /// Maximum size constraint
    pub max_size: f32,

    /// Track type
    pub track_type: TrackType,
}

/// Track sizing function
#[derive(Debug, Clone)]
pub struct TrackSizingFunction {
    /// Sizing function type
    pub function_type: SizingFunctionType,

    /// Size values
    pub sizes: Vec<f32>,

    /// Flex factor if applicable
    pub flex_factor: Option<f32>,
}

/// Track type enumeration
#[derive(Debug, Clone)]
pub enum TrackType {
    /// Fixed size track
    Fixed,

    /// Flexible size track
    Flexible,

    /// Minimum content size
    MinContent,

    /// Maximum content size
    MaxContent,

    /// Auto-sized track
    Auto,
}

/// Sizing function types
#[derive(Debug, Clone)]
pub enum SizingFunctionType {
    /// Fixed size
    Fixed(f32),

    /// Minimum/maximum size
    MinMax(f32, f32),

    /// Flexible unit
    Fr(f32),

    /// Fit content
    FitContent(f32),

    /// Repeat function
    Repeat(u32, Vec<SizingFunctionType>),
}

/// Subgrid layout result
#[derive(Debug, Clone)]
pub struct SubgridLayoutResult {
    /// Final item placements
    pub item_placements: Vec<ItemPlacement>,

    /// Track size contributions to parent
    pub size_contributions: Vec<TrackSizeContribution>,

    /// Layout success status
    pub success: bool,

    /// Layout warnings
    pub warnings: Vec<String>,
}
