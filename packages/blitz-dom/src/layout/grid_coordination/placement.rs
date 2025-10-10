//! Auto-placement algorithms for CSS Grid Layout

use taffy::NodeId;

use super::super::grid_errors::GridPreprocessingError;
use super::placement_types::*;
use super::types::*;

impl GridLayoutCoordinator {
    /// Enhanced auto-placement implementation with CSS order processing
    pub fn auto_place_items<Tree>(
        &self,
        _subgrid_id: NodeId,
        placement_state: &mut AutoPlacementState,
        tree: &Tree,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError>
    where
        Tree: taffy::LayoutGridContainer,
    {
        let mut placements = Vec::new();

        // Items are already sorted by CSS order in placement_state.ordered_items
        // (sorted by get_items_in_css_order() before this is called)

        // Collect items to avoid borrow checker issues
        let items_to_process: Vec<_> = placement_state
            .ordered_items
            .iter()
            .filter(|(item_id, _)| !placement_state.explicit_placements.contains_key(item_id))
            .map(|(item_id, _)| *item_id)
            .collect();

        for item_id in items_to_process {
            // Get item's grid spans from style
            let (row_span, col_span) = self.get_item_spans(item_id, tree)?;

            // Find placement using cursor
            let position = self.find_placement_from_cursor(
                placement_state,
                row_span,
                col_span,
            )?;

            // Create placement
            let placement = ItemPlacement {
                node_id: item_id,
                grid_area: GridArea {
                    row_start: position.row,
                    row_end: position.row + row_span as i32,
                    column_start: position.column,
                    column_end: position.column + col_span as i32,
                },
                placement_method: PlacementMethod::AutoPlacement,
            };

            // Update track occupancy
            placement_state.track_occupancy.mark_area_occupied(&placement);

            // Advance cursor based on flow direction
            self.advance_cursor_after_placement(placement_state, &placement, row_span, col_span)?;

            placements.push(placement);
        }

        Ok(placements)
    }

    /// Get item spans from grid style
    fn get_item_spans<Tree>(
        &self,
        item_id: NodeId,
        tree: &Tree,
    ) -> Result<(usize, usize), GridPreprocessingError>
    where
        Tree: taffy::LayoutGridContainer,
    {
        use taffy::GridItemStyle;
        use taffy::GridPlacement;

        let style = tree.get_grid_child_style(item_id);

        // Get row span
        let row_span = match (style.grid_row().start, style.grid_row().end) {
            (GridPlacement::Span(n), _) => n as usize,
            (_, GridPlacement::Span(n)) => n as usize,
            _ => 1, // Default span
        };

        // Get column span
        let col_span = match (style.grid_column().start, style.grid_column().end) {
            (GridPlacement::Span(n), _) => n as usize,
            (_, GridPlacement::Span(n)) => n as usize,
            _ => 1, // Default span
        };

        Ok((row_span, col_span))
    }

    /// Find placement position from current cursor
    fn find_placement_from_cursor(
        &self,
        placement_state: &AutoPlacementState,
        row_span: usize,
        col_span: usize,
    ) -> Result<GridPosition, GridPreprocessingError> {
        // Use TrackOccupancyMap to find next available position
        let position = placement_state.track_occupancy.find_next_available(
            row_span,
            col_span,
            placement_state.cursor_position.row,
            placement_state.cursor_position.column,
        );

        if let Some(pos) = position {
            Ok(pos)
        } else {
            // If no position found, try from beginning (grid might need to grow)
            // For now, return error - proper implementation would grow the grid
            Err(GridPreprocessingError::PreprocessingFailed {
                operation: "auto_placement".to_string(),
                node_id: 0,
                details: "No available position found for auto-placement".to_string(),
            })
        }
    }

    /// Advance cursor after placement based on flow direction
    fn advance_cursor_after_placement(
        &self,
        placement_state: &mut AutoPlacementState,
        placement: &ItemPlacement,
        _row_span: usize,
        _col_span: usize,
    ) -> Result<(), GridPreprocessingError> {
        use super::placement_types::FlowDirection;

        // Advance cursor based on grid-auto-flow property
        match placement_state.flow_direction {
            FlowDirection::Row => {
                // Row flow: cursor moves column-by-column, then to next row
                // Move to column after the placed item
                placement_state.cursor_position.column = placement.grid_area.column_end;

                // If past grid width, move to next row and column 0
                if placement_state.cursor_position.column >= placement_state.track_occupancy.grid_size.column {
                    placement_state.cursor_position.column = 0;
                    placement_state.cursor_position.row += 1;
                }
            }
            FlowDirection::Column => {
                // Column flow: cursor moves row-by-row, then to next column
                // Move to row after the placed item
                placement_state.cursor_position.row = placement.grid_area.row_end;

                // If past grid height, move to next column and row 0
                if placement_state.cursor_position.row >= placement_state.track_occupancy.grid_size.row {
                    placement_state.cursor_position.row = 0;
                    placement_state.cursor_position.column += 1;
                }
            }
        }

        Ok(())
    }

    /// Dense packing implementation for backfill algorithm
    pub fn dense_packing_pass<Tree>(
        &self,
        _subgrid_id: NodeId,
        placement_state: &mut AutoPlacementState,
        tree: &Tree,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError>
    where
        Tree: taffy::LayoutGridContainer,
    {
        // Check if dense packing is enabled
        let dense_enabled = placement_state
            .dense_packing_state
            .as_ref()
            .map(|ds| ds.enabled)
            .unwrap_or(false);

        if !dense_enabled {
            return Ok(Vec::new());
        }

        let mut dense_placements = Vec::new();

        // Get pending items that haven't been placed yet
        let pending_items: Vec<NodeId> = placement_state
            .dense_packing_state
            .as_ref()
            .map(|ds| ds.pending_items.clone())
            .unwrap_or_default();

        for item_id in pending_items {
            // Get item spans
            let (row_span, col_span) = self.get_item_spans(item_id, tree)?;

            // KEY DIFFERENCE: Search from position (0, 0) for EACH item
            // Don't use the cursor - always start from beginning
            let position = self.find_first_available_position(
                placement_state,
                row_span,
                col_span,
            )?;

            if let Some(pos) = position {
                let placement = ItemPlacement {
                    node_id: item_id,
                    grid_area: GridArea {
                        row_start: pos.row,
                        row_end: pos.row + row_span as i32,
                        column_start: pos.column,
                        column_end: pos.column + col_span as i32,
                    },
                    placement_method: PlacementMethod::DensePacking,
                };

                placement_state.track_occupancy.mark_area_occupied(&placement);
                dense_placements.push(placement);
            }
        }

        Ok(dense_placements)
    }

    /// Find first available position searching from beginning (for dense packing)
    fn find_first_available_position(
        &self,
        state: &AutoPlacementState,
        row_span: usize,
        col_span: usize,
    ) -> Result<Option<GridPosition>, GridPreprocessingError> {
        use super::placement_types::FlowDirection;

        let grid_size = &state.track_occupancy.grid_size;

        // For dense mode: Always search from (0, 0) to fill gaps
        // Respect flow direction even in dense mode to maintain consistent search order
        match state.flow_direction {
            FlowDirection::Row => {
                // Row flow: iterate columns within rows
                for row in 0..grid_size.row {
                    for col in 0..grid_size.column {
                        let row_end = row + row_span as i32;
                        let col_end = col + col_span as i32;

                        // Check bounds
                        if row_end > grid_size.row || col_end > grid_size.column {
                            continue;
                        }

                        // Check if area is available
                        if state.track_occupancy.is_area_available(row, row_end, col, col_end) {
                            return Ok(Some(GridPosition { row, column: col }));
                        }
                    }
                }
            }
            FlowDirection::Column => {
                // Column flow: iterate rows within columns
                for col in 0..grid_size.column {
                    for row in 0..grid_size.row {
                        let row_end = row + row_span as i32;
                        let col_end = col + col_span as i32;

                        // Check bounds
                        if row_end > grid_size.row || col_end > grid_size.column {
                            continue;
                        }

                        // Check if area is available
                        if state.track_occupancy.is_area_available(row, row_end, col, col_end) {
                            return Ok(Some(GridPosition { row, column: col }));
                        }
                    }
                }
            }
        }

        Ok(None) // No available position found
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
        use super::placement_types::FlowDirection;

        match placement_state.flow_direction {
            FlowDirection::Row => {
                // Row flow: advance column, then row
                position.column += 1;
                if position.column >= placement_state.track_occupancy.grid_size.column {
                    position.column = 0;
                    position.row += 1;
                }
            }
            FlowDirection::Column => {
                // Column flow: advance row, then column
                position.row += 1;
                if position.row >= placement_state.track_occupancy.grid_size.row {
                    position.row = 0;
                    position.column += 1;
                }
            }
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
        use super::placement_types::FlowDirection;

        // Advance cursor based on placement and flow direction
        match placement_state.flow_direction {
            FlowDirection::Row => {
                // Row flow: advance to column after placement
                placement_state.cursor_position.column = placement.grid_area.column_end;
                if placement_state.cursor_position.column
                    >= placement_state.track_occupancy.grid_size.column
                {
                    placement_state.cursor_position.column = 0;
                    placement_state.cursor_position.row += 1;
                }
            }
            FlowDirection::Column => {
                // Column flow: advance to row after placement
                placement_state.cursor_position.row = placement.grid_area.row_end;
                if placement_state.cursor_position.row
                    >= placement_state.track_occupancy.grid_size.row
                {
                    placement_state.cursor_position.row = 0;
                    placement_state.cursor_position.column += 1;
                }
            }
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
