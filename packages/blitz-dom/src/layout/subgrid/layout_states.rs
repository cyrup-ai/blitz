//! Layout state definitions for grid coordination
//!
//! This module contains the state enums and structs that track the various
//! phases of CSS Grid Level 2 and Level 3 layout coordination.

use std::collections::HashMap;

use taffy::prelude::*;

use super::types::*;

/// Layout pass coordination state
#[derive(Debug, Clone)]
pub enum LayoutPassState {
    /// Initial pass: Auto-placement and track count determination
    InitialPlacement {
        /// Items processed in CSS order
        items_by_order: Vec<NodeId>,
        /// Current auto-placement cursor position
        auto_cursor: AutoPlacementCursor,
        /// Track counts determined so far
        determined_track_counts: DeterminedTrackCounts,
    },
    /// Intrinsic sizing pass: Resolve content-based track sizes
    IntrinsicSizing {
        /// Items contributing to intrinsic sizes
        contributing_items: Vec<NodeId>,
        /// Track size calculations in progress
        track_size_calculations: TrackSizeCalculations,
        /// Bidirectional sizing coordination state
        bidirectional_state: BidirectionalSizingState,
    },
    /// Final layout pass: Position items with resolved tracks
    FinalLayout {
        /// Resolved track sizes for layout
        resolved_tracks: ResolvedTrackSizes,
        /// Item positions and spans
        item_placements: HashMap<NodeId, ItemPlacement>,
        /// Masonry layout coordination if applicable
        masonry_coordination: Option<MasonryCoordinationState>,
    },
}

/// Subgrid layout coordination state
#[derive(Debug, Clone)]
pub struct SubgridLayoutState {
    /// Parent grid this subgrid inherits from
    pub parent_grid_id: Option<NodeId>,
    /// Inheritance data with track and line name mapping
    pub inheritance_data: SubgridTrackInheritance,
    /// Current coordination pass
    pub coordination_pass: usize,
    /// Subgrid size propagation state
    pub subgrid_propagation: SubgridSizePropagation,
}

/// CSS Grid Level 3 masonry layout state
#[derive(Debug, Clone)]
pub struct MasonryLayoutState {
    /// Masonry axis (usually block axis)
    pub masonry_axis: AbstractAxis,
    /// Grid axis (perpendicular to masonry axis)
    pub grid_axis: AbstractAxis,
    /// Track positions for masonry algorithm
    pub track_positions: Vec<f32>,
    /// Item heights for masonry packing
    pub item_sizes: HashMap<NodeId, f32>,
    /// Masonry packing algorithm state
    pub packing_state: MasonryPackingState,
}

/// Auto-placement cursor tracking position in grid
#[derive(Debug, Clone)]
pub struct AutoPlacementCursor {
    /// Current row position
    pub row: usize,
    /// Current column position
    pub column: usize,
    /// Direction for next placement attempt
    pub direction: AutoFlowDirection,
}

/// Determined track counts from auto-placement
#[derive(Debug, Clone)]
pub struct DeterminedTrackCounts {
    /// Number of explicit rows
    pub explicit_rows: usize,
    /// Number of explicit columns
    pub explicit_columns: usize,
    /// Number of implicit rows created
    pub implicit_rows: usize,
    /// Number of implicit columns created
    pub implicit_columns: usize,
}

/// Track size calculations for intrinsic sizing
#[derive(Debug, Clone)]
pub struct TrackSizeCalculations {
    /// Min content sizes for each track
    pub min_content_sizes: Vec<f32>,
    /// Max content sizes for each track
    pub max_content_sizes: Vec<f32>,
    /// Flexible track growth calculations
    pub flex_calculations: Vec<FlexCalculation>,
}

/// Flexible track calculation state
#[derive(Debug, Clone)]
pub struct FlexCalculation {
    /// Track index
    pub track_index: usize,
    /// Flex factor
    pub flex_factor: f32,
    /// Base size
    pub base_size: f32,
    /// Growth limit
    pub growth_limit: Option<f32>,
}

/// Bidirectional sizing coordination between row and column axes
#[derive(Debug, Clone)]
pub struct BidirectionalSizingState {
    /// Dependencies from row sizing to column sizing
    pub row_to_column_deps: Vec<SizingDependency>,
    /// Dependencies from column sizing to row sizing
    pub column_to_row_deps: Vec<SizingDependency>,
    /// Current coordination pass number
    pub coordination_pass: usize,
    /// Maximum coordination passes allowed
    pub max_coordination_passes: usize,
}

/// Resolved track sizes for final layout
#[derive(Debug, Clone)]
pub struct ResolvedTrackSizes {
    /// Final row track sizes
    pub row_sizes: Vec<f32>,
    /// Final column track sizes
    pub column_sizes: Vec<f32>,
    /// Row line positions
    pub row_positions: Vec<f32>,
    /// Column line positions
    pub column_positions: Vec<f32>,
}

/// Masonry coordination state
#[derive(Debug, Clone)]
pub struct MasonryCoordinationState {
    /// Masonry packing positions
    pub packing_positions: Vec<MasonryPosition>,
    /// Track height tracking for masonry
    pub track_heights: Vec<f32>,
    /// Masonry flow direction
    pub flow_direction: MasonryFlowDirection,
}

/// Masonry position for items
#[derive(Debug, Clone)]
pub struct MasonryPosition {
    /// Grid track
    pub track: usize,
    /// Position within track
    pub position: f32,
    /// Item associated with position
    pub item_id: taffy::prelude::NodeId,
}

/// Masonry flow direction
#[derive(Debug, Clone, Copy)]
pub enum MasonryFlowDirection {
    /// Pack items starting from first track
    Pack,
    /// Pack items starting from last track
    Next,
}

/// Sizing dependency between grid axes
#[derive(Debug, Clone)]
pub struct SizingDependency {
    /// Source axis that affects sizing
    pub source_axis: AbstractAxis,
    /// Target axis that is affected
    pub target_axis: AbstractAxis,
    /// Items involved in dependency
    pub dependent_items: Vec<NodeId>,
    /// Dependency strength
    pub strength: DependencyStrength,
}

/// Subgrid size propagation state
#[derive(Debug, Clone)]
pub struct SubgridSizePropagation {
    /// Subgrids affecting this grid
    pub affecting_subgrids: Vec<NodeId>,
    /// Propagation direction
    pub propagation_direction: PropagationDirection,
    /// Current propagation phase
    pub propagation_phase: PropagationPhase,
}

/// Masonry packing state
#[derive(Debug, Clone)]
pub struct MasonryPackingState {
    /// Next item to pack
    pub next_item_index: usize,
    /// Current masonry axis position
    pub current_masonry_position: f32,
    /// Packing strategy
    pub packing_strategy: MasonryPackingStrategy,
}

/// Auto-placement state for grid containers
#[derive(Debug, Clone)]
pub struct AutoPlacementState {
    /// Current placement cursor
    pub cursor: AutoPlacementCursor,
    /// Items remaining to place
    pub pending_items: Vec<NodeId>,
    /// Items already placed
    pub placed_items: Vec<NodeId>,
    /// Track occupancy state
    pub track_occupancy: TrackOccupancyState,
}

/// Intrinsic sizing state for bidirectional coordination
#[derive(Debug, Clone)]
pub struct IntrinsicSizingState {
    /// Row axis sizing state
    pub row_sizing: AxisSizingState,
    /// Column axis sizing state
    pub column_sizing: AxisSizingState,
    /// Cross-axis dependencies
    pub cross_axis_deps: Vec<CrossAxisDependency>,
    /// Current coordination pass
    pub coordination_pass: usize,
    /// Previous row track sizes for convergence checking
    pub previous_row_sizes: Option<Vec<f32>>,
    /// Previous column track sizes for convergence checking
    pub previous_column_sizes: Option<Vec<f32>>,
}

/// Axis sizing state for intrinsic sizing
#[derive(Debug, Clone)]
pub struct AxisSizingState {
    /// Track sizes being calculated
    pub track_sizes: Vec<f32>,
    /// Sizing constraints
    pub sizing_constraints: Vec<SizingConstraint>,
    /// Flexible tracks
    pub flexible_tracks: Vec<FlexibleTrack>,
}

/// Track occupancy tracking for auto-placement
#[derive(Debug, Clone)]
pub struct TrackOccupancyState {
    /// Occupied grid cells
    pub occupied_cells: HashMap<(usize, usize), NodeId>,
    /// Row occupancy bits
    pub row_occupancy: Vec<u64>,
    /// Column occupancy bits
    pub column_occupancy: Vec<u64>,
}

/// Auto-flow direction for grid-auto-flow property
#[derive(Debug, Clone, Copy)]
pub enum AutoFlowDirection {
    /// Flow in row direction (grid-auto-flow: row)
    Row,
    /// Flow in column direction (grid-auto-flow: column)
    Column,
}

/// Abstract axis for bidirectional coordination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstractAxis {
    /// Block axis (typically vertical)
    Block,
    /// Inline axis (typically horizontal)
    Inline,
}

/// Masonry packing strategy
#[derive(Debug, Clone, Copy)]
pub enum MasonryPackingStrategy {
    /// Pack into shortest track
    Shortest,
    /// Pack in order
    InOrder,
    /// Pack with balancing
    Balanced,
}

/// Dependency strength for coordination
#[derive(Debug, Clone, Copy)]
pub enum DependencyStrength {
    /// Weak dependency
    Weak,
    /// Strong dependency
    Strong,
    /// Required dependency
    Required,
}

/// Propagation direction for subgrid sizes
#[derive(Debug, Clone, Copy)]
pub enum PropagationDirection {
    /// Parent to child
    ParentToChild,
    /// Child to parent
    ChildToParent,
    /// Bidirectional
    Bidirectional,
}

/// Propagation phase
#[derive(Debug, Clone, Copy)]
pub enum PropagationPhase {
    /// Initial phase
    Initial,
    /// Content measurement
    ContentMeasurement,
    /// Size resolution
    SizeResolution,
    /// Final layout
    FinalLayout,
}

/// Cross-axis dependency for intrinsic sizing
#[derive(Debug, Clone)]
pub struct CrossAxisDependency {
    /// Source axis
    pub source_axis: AbstractAxis,
    /// Target axis
    pub target_axis: AbstractAxis,
    /// Items creating dependency
    pub dependent_items: Vec<NodeId>,
    /// Dependency type
    pub dependency_type: DependencyType,
}

/// Sizing constraint for track sizing
#[derive(Debug, Clone)]
pub struct SizingConstraint {
    /// Track index
    pub track_index: usize,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Constraint value
    pub value: f32,
}

/// Flexible track definition
#[derive(Debug, Clone)]
pub struct FlexibleTrack {
    /// Track index
    pub track_index: usize,
    /// Flex factor
    pub flex_factor: f32,
    /// Minimum size
    pub min_size: f32,
    /// Maximum size
    pub max_size: Option<f32>,
}

/// Dependency type for axis dependencies
#[derive(Debug, Clone, Copy)]
pub enum DependencyType {
    /// Content size affects other axis
    ContentSize,
    /// Aspect ratio constraint
    AspectRatio,
    /// Intrinsic ratio dependency
    IntrinsicRatio,
}

/// Constraint type for sizing constraints
#[derive(Debug, Clone, Copy)]
pub enum ConstraintType {
    /// Minimum constraint
    Minimum,
    /// Maximum constraint
    Maximum,
    /// Fixed constraint
    Fixed,
    /// Preferred constraint
    Preferred,
}
