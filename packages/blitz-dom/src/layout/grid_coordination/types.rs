//! Core data structures for CSS Grid Multi-Pass Layout Coordination System

use std::collections::HashMap;

use taffy::NodeId;

use super::super::grid_context::GridAxis;
use super::super::subgrid::layout_states::IntrinsicSizingState as SubgridIntrinsicSizingState;

/// Multi-pass layout coordination system for CSS Grid
#[derive(Debug, Clone)]
pub struct GridLayoutCoordinator {
    /// Multi-pass layout state management
    pub layout_passes: HashMap<NodeId, LayoutPassState>,

    /// Subgrid layout states per node
    pub subgrid_states: HashMap<NodeId, SubgridLayoutState>,

    /// Auto-placement coordination
    pub auto_placement_states: HashMap<NodeId, AutoPlacementState>,

    /// Intrinsic sizing coordination
    pub intrinsic_sizing_states: HashMap<NodeId, SubgridIntrinsicSizingState>,

    /// CSS Grid Level 3 masonry support
    pub masonry_states: HashMap<NodeId, MasonryLayoutState>,
}

/// Layout pass state tracking for multi-pass coordination
#[derive(Debug, Clone)]
pub struct LayoutPassState {
    /// Current pass number (1-4)
    pub current_pass: u8,

    /// Pass completion status
    pub passes_completed: Vec<bool>,

    /// Layout dependencies tracking
    pub dependencies: Vec<NodeId>,

    /// Coordination flags
    pub requires_parent_recompute: bool,
    pub has_size_changes: bool,
}

/// Subgrid layout state management
#[derive(Debug, Clone)]
pub struct SubgridLayoutState {
    /// Track definitions inherited from parent (replaces grid-template-*)
    pub inherited_tracks: InheritedTrackDefinitions,

    /// Line name mapping (parent + local names)
    pub line_name_mapping: LineNameMap,

    /// Current intrinsic size contributions to parent tracks
    pub size_contributions: Vec<TrackSizeContribution>,

    /// Layout pass coordination state
    pub layout_pass_state: LayoutPassState,
}

/// Intrinsic sizing coordination state
#[derive(Debug, Clone)]
pub struct IntrinsicSizingState {
    /// Content size contributions per item
    pub content_contributions: HashMap<NodeId, IntrinsicSizeContribution>,

    /// Track size requirements from all items
    pub track_size_requirements: Vec<TrackSizeRequirement>,

    /// Multi-pass coordination state
    pub sizing_pass_state: SizingPassState,
}

/// CSS Grid Level 3 masonry layout state
#[derive(Debug, Clone)]
pub struct MasonryLayoutState {
    /// Running positions for track selection
    pub track_running_positions: Vec<f32>,

    /// Item tolerance for tie-breaking (default: 1em)
    pub item_tolerance: f32,

    /// Virtual masonry items for performance
    pub virtual_items: Vec<VirtualMasonryItem>,
}

/// Track definitions inherited from parent grid
#[derive(Debug, Clone)]
pub struct InheritedTrackDefinitions {
    /// Row track definitions from parent
    pub row_tracks: Vec<TrackDefinition>,

    /// Column track definitions from parent
    pub column_tracks: Vec<TrackDefinition>,

    /// Track sizing functions
    pub row_sizing_functions: Vec<TrackSizingFunction>,
    pub column_sizing_functions: Vec<TrackSizingFunction>,
}

/// Replaced grid template properties ready for application
#[derive(Debug, Clone)]
pub struct ReplacedGridTemplates {
    /// Row track sizing functions converted to Taffy format
    pub row_functions: Vec<taffy::TrackSizingFunction>,
    
    /// Column track sizing functions converted to Taffy format
    pub column_functions: Vec<taffy::TrackSizingFunction>,
}

/// Line name mapping for parent and local names
#[derive(Debug, Clone)]
pub struct LineNameMap {
    /// Parent grid line names mapped to subgrid coordinates
    pub parent_line_names: HashMap<String, Vec<i32>>,

    /// Local subgrid line names
    pub local_line_names: HashMap<String, Vec<i32>>,

    /// Combined mapping for resolution
    pub combined_mapping: HashMap<String, Vec<i32>>,
}

/// Track size contribution from subgrid items
#[derive(Debug, Clone)]
pub struct TrackSizeContribution {
    /// Target track index in parent grid
    pub parent_track_index: usize,

    /// Axis (row or column)
    pub axis: GridAxis,

    /// Minimum size contribution
    pub min_size: f32,

    /// Maximum size contribution
    pub max_size: f32,

    /// Preferred size contribution
    pub preferred_size: f32,
}

/// Intrinsic size contribution per item
#[derive(Debug, Clone)]
pub struct IntrinsicSizeContribution {
    /// Node contributing the size
    pub node_id: NodeId,

    /// Minimum content size
    pub min_content_size: f32,

    /// Maximum content size
    pub max_content_size: f32,

    /// Affected tracks
    pub affected_tracks: Vec<usize>,

    /// Contribution axis
    pub axis: GridAxis,
}

/// Track size requirement definition
#[derive(Debug, Clone)]
pub struct TrackSizeRequirement {
    /// Track index
    pub track_index: usize,

    /// Required minimum size
    pub min_size: f32,

    /// Required maximum size
    pub max_size: f32,

    /// Flexible growth factor
    pub flex_factor: f32,

    /// Axis for this requirement
    pub axis: GridAxis,
}

/// Sizing pass state for intrinsic coordination
#[derive(Debug, Clone)]
pub struct SizingPassState {
    /// Current sizing pass
    pub current_pass: u8,

    /// Content measurement completed
    pub content_measured: bool,

    /// Track size distribution completed
    pub tracks_distributed: bool,

    /// Final sizes computed
    pub final_sizes_computed: bool,
}

/// Virtual masonry item for performance optimization
#[derive(Debug, Clone)]
pub struct VirtualMasonryItem {
    /// Item node ID
    pub node_id: NodeId,

    /// Virtual position for track sizing
    pub virtual_position: GridPosition,

    /// Item dimensions
    pub item_size: (f32, f32), // (width, height)

    /// Track preference
    pub preferred_track: Option<usize>,
}

// Re-exports from placement_types
pub use super::placement_types::{
    AutoPlacementState, DensePackingState, GridArea, GridPosition, ItemPlacement, PlacementMethod,
    TrackOccupancyMap,
};
// Re-exports from track_types
pub use super::track_types::{
    SizingFunctionType, SubgridLayoutResult, TrackDefinition, TrackSizingFunction, TrackType,
};

// Default implementations

impl Default for GridLayoutCoordinator {
    fn default() -> Self {
        Self {
            layout_passes: HashMap::new(),
            subgrid_states: HashMap::new(),
            auto_placement_states: HashMap::new(),
            intrinsic_sizing_states: HashMap::new(),
            masonry_states: HashMap::new(),
        }
    }
}

impl Default for LayoutPassState {
    fn default() -> Self {
        Self {
            current_pass: 1,
            passes_completed: vec![false; 4],
            dependencies: Vec::new(),
            requires_parent_recompute: false,
            has_size_changes: false,
        }
    }
}

impl Default for SizingPassState {
    fn default() -> Self {
        Self {
            current_pass: 1,
            content_measured: false,
            tracks_distributed: false,
            final_sizes_computed: false,
        }
    }
}

impl GridLayoutCoordinator {
    /// Create a new grid layout coordinator
    pub fn new() -> Self {
        Self::default()
    }
}
