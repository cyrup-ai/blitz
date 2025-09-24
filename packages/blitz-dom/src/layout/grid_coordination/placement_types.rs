//! Auto-placement related data structures for CSS Grid Layout

use std::collections::HashMap;

use taffy::NodeId;

/// Auto-placement state management for CSS Grid algorithm
#[derive(Debug, Clone)]
pub struct AutoPlacementState {
    /// Current placement cursor position
    pub cursor_position: GridPosition,

    /// Items processed in CSS order
    pub ordered_items: Vec<(NodeId, i32)>, // (node_id, order_value)

    /// Explicit placements (affect cursor movement)
    pub explicit_placements: HashMap<NodeId, GridArea>,

    /// Dense packing state for backfill
    pub dense_packing_state: Option<DensePackingState>,

    /// Track occupancy for conflict detection
    pub track_occupancy: TrackOccupancyMap,
}

/// Grid position for placement coordination
#[derive(Debug, Clone, Copy)]
pub struct GridPosition {
    /// Row coordinate
    pub row: i32,

    /// Column coordinate
    pub column: i32,
}

/// Grid area definition for explicit placement
#[derive(Debug, Clone)]
pub struct GridArea {
    /// Row start line
    pub row_start: i32,

    /// Row end line
    pub row_end: i32,

    /// Column start line
    pub column_start: i32,

    /// Column end line
    pub column_end: i32,
}

/// Dense packing state for auto-placement backfill
#[derive(Debug, Clone)]
pub struct DensePackingState {
    /// Unfilled grid positions
    pub unfilled_positions: Vec<GridPosition>,

    /// Items awaiting dense placement
    pub pending_items: Vec<NodeId>,

    /// Dense packing enabled flag
    pub enabled: bool,
}

/// Track occupancy map for conflict detection
#[derive(Debug, Clone)]
pub struct TrackOccupancyMap {
    /// Occupied cells in grid
    pub occupied_cells: HashMap<(i32, i32), NodeId>,

    /// Grid size tracking
    pub grid_size: GridPosition,
}

/// Item placement result
#[derive(Debug, Clone)]
pub struct ItemPlacement {
    /// Item node ID
    pub node_id: NodeId,

    /// Placement area
    pub grid_area: GridArea,

    /// Placement method used
    pub placement_method: PlacementMethod,
}

/// Placement method enumeration
#[derive(Debug, Clone)]
pub enum PlacementMethod {
    /// Explicit placement via properties
    Explicit,

    /// Auto-placement algorithm
    AutoPlacement,

    /// Dense packing backfill
    DensePacking,
}

// Default implementations

impl Default for GridPosition {
    fn default() -> Self {
        Self { row: 0, column: 0 }
    }
}

impl Default for TrackOccupancyMap {
    fn default() -> Self {
        Self {
            occupied_cells: HashMap::new(),
            grid_size: GridPosition::default(),
        }
    }
}
