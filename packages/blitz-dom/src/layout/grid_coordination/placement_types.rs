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

impl TrackOccupancyMap {
    /// Check if a rectangular area is available for placement
    pub fn is_area_available(&self, row_start: i32, row_end: i32, col_start: i32, col_end: i32) -> bool {
        // Check if area is within grid bounds
        if row_start < 0 || col_start < 0 {
            return false;
        }

        // Check all cells in the rectangular area
        for row in row_start..row_end {
            for col in col_start..col_end {
                if self.occupied_cells.contains_key(&(row, col)) {
                    return false;
                }
            }
        }
        true
    }

    /// Mark area as occupied by an item
    pub fn mark_area_occupied(&mut self, placement: &ItemPlacement) {
        for row in placement.grid_area.row_start..placement.grid_area.row_end {
            for col in placement.grid_area.column_start..placement.grid_area.column_end {
                self.occupied_cells.insert((row, col), placement.node_id);
            }
        }
    }

    /// Find next available position for an item with given spans
    pub fn find_next_available(&self, row_span: usize, col_span: usize, start_row: i32, start_col: i32) -> Option<GridPosition> {
        // Search from start position for first area that fits the item
        let max_row = self.grid_size.row;
        let max_col = self.grid_size.column;

        // Try each position starting from the given start position
        for row in start_row..max_row {
            let col_start = if row == start_row { start_col } else { 0 };

            for col in col_start..max_col {
                // Check if item fits at this position
                let row_end = row + row_span as i32;
                let col_end = col + col_span as i32;

                // Check bounds
                if row_end > max_row || col_end > max_col {
                    continue;
                }

                // Check if area is available
                if self.is_area_available(row, row_end, col, col_end) {
                    return Some(GridPosition { row, column: col });
                }
            }
        }

        None // No space available
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
