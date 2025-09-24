//! Auto-placement algorithms for CSS Grid Layout

use taffy::NodeId;

use super::super::grid_errors::GridPreprocessingError;
use super::placement_types::*;
use super::types::*;

impl GridLayoutCoordinator {
    /// Enhanced auto-placement implementation with CSS order processing
    pub fn auto_place_items(
        &self,
        _subgrid_id: NodeId,
        placement_state: &mut AutoPlacementState,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError> {
        let mut placements = Vec::new();

        // Process items in CSS order (sorted by order property)
        placement_state
            .ordered_items
            .sort_by_key(|(_, order)| *order);

        // Collect items to avoid borrow checker issues
        let items_to_process: Vec<_> = placement_state
            .ordered_items
            .iter()
            .filter(|(item_id, _)| !placement_state.explicit_placements.contains_key(item_id))
            .map(|(item_id, _)| *item_id)
            .collect();

        for item_id in items_to_process {
            // Find placement using cursor algorithm
            let placement = self.find_auto_placement(item_id, placement_state)?;
            placements.push(placement.clone());

            // Update track occupancy
            self.update_track_occupancy(&placement, placement_state)?;

            // Advance placement cursor
            self.advance_placement_cursor(placement_state, &placement)?;
        }

        Ok(placements)
    }

    /// Dense packing implementation for backfill algorithm
    pub fn dense_packing_pass(
        &self,
        _subgrid_id: NodeId,
        placement_state: &mut AutoPlacementState,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError> {
        let dense_state_enabled = placement_state
            .dense_packing_state
            .as_ref()
            .map(|ds| ds.enabled)
            .unwrap_or(false);

        if !dense_state_enabled {
            return Ok(Vec::new());
        }

        let mut dense_placements = Vec::new();

        // Collect pending items to avoid borrowing issues
        let pending_items: Vec<NodeId> = placement_state
            .dense_packing_state
            .as_ref()
            .map(|ds| ds.pending_items.clone())
            .unwrap_or_default();

        // Process pending items for dense packing
        for item_id in pending_items {
            // Try to find placement in unfilled positions
            if let Some(dense_state) = &mut placement_state.dense_packing_state {
                if let Some(position) = self.find_dense_placement(item_id, dense_state)? {
                    let placement = ItemPlacement {
                        node_id: item_id,
                        grid_area: GridArea {
                            row_start: position.row,
                            row_end: position.row + 1,
                            column_start: position.column,
                            column_end: position.column + 1,
                        },
                        placement_method: PlacementMethod::DensePacking,
                    };

                    dense_placements.push(placement.clone());
                    self.update_track_occupancy(&placement, placement_state)?;
                }
            }
        }

        Ok(dense_placements)
    }

    /// Find auto placement for an item
    pub fn find_auto_placement(
        &self,
        item_id: NodeId,
        placement_state: &AutoPlacementState,
    ) -> Result<ItemPlacement, GridPreprocessingError> {
        // Start from current cursor position
        let mut current_position = placement_state.cursor_position;
        let grid_size = &placement_state.track_occupancy.grid_size;
        
        // Calculate maximum possible positions to prevent infinite loops
        let max_positions = (grid_size.row * grid_size.column).max(1);
        let mut attempts = 0;

        // Find first available position with bounds checking
        while self.is_position_occupied(&current_position, placement_state) {
            attempts += 1;
            
            // Prevent infinite loops if grid is completely full
            if attempts >= max_positions {
                return Err(GridPreprocessingError::PreprocessingFailed {
                    operation: "auto_placement".to_string(),
                    node_id: item_id.into(),
                    details: format!("No available positions found after checking {} positions", max_positions),
                });
            }
            
            current_position = self.advance_position(current_position, placement_state)?;
        }

        Ok(ItemPlacement {
            node_id: item_id,
            grid_area: GridArea {
                row_start: current_position.row,
                row_end: current_position.row + 1,
                column_start: current_position.column,
                column_end: current_position.column + 1,
            },
            placement_method: PlacementMethod::AutoPlacement,
        })
    }

    /// Check if a position is occupied
    pub fn is_position_occupied(
        &self,
        position: &GridPosition,
        placement_state: &AutoPlacementState,
    ) -> bool {
        placement_state
            .track_occupancy
            .occupied_cells
            .contains_key(&(position.row, position.column))
    }

    /// Advance position for cursor movement
    pub fn advance_position(
        &self,
        mut position: GridPosition,
        placement_state: &AutoPlacementState,
    ) -> Result<GridPosition, GridPreprocessingError> {
        position.column += 1;
        if position.column >= placement_state.track_occupancy.grid_size.column {
            position.column = 0;
            position.row += 1;
        }
        Ok(position)
    }

    /// Update track occupancy with a placement
    pub fn update_track_occupancy(
        &self,
        placement: &ItemPlacement,
        placement_state: &mut AutoPlacementState,
    ) -> Result<(), GridPreprocessingError> {
        for row in placement.grid_area.row_start..placement.grid_area.row_end {
            for col in placement.grid_area.column_start..placement.grid_area.column_end {
                placement_state
                    .track_occupancy
                    .occupied_cells
                    .insert((row, col), placement.node_id);
            }
        }
        Ok(())
    }

    /// Advance placement cursor based on placement
    pub fn advance_placement_cursor(
        &self,
        placement_state: &mut AutoPlacementState,
        placement: &ItemPlacement,
    ) -> Result<(), GridPreprocessingError> {
        // Advance cursor based on placement
        placement_state.cursor_position.column = placement.grid_area.column_end;
        if placement_state.cursor_position.column
            >= placement_state.track_occupancy.grid_size.column
        {
            placement_state.cursor_position.column = 0;
            placement_state.cursor_position.row += 1;
        }
        Ok(())
    }

    /// Find dense placement for an item
    pub fn find_dense_placement(
        &self,
        _item_id: NodeId,
        dense_state: &DensePackingState,
    ) -> Result<Option<GridPosition>, GridPreprocessingError> {
        // Try to place item in first available unfilled position
        // Check if position is still available
        // This would integrate with full occupancy tracking
        Ok(dense_state.unfilled_positions.first().copied())
    }
}
