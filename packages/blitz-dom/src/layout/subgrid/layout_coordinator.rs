//! Core CSS Grid layout coordination system
//!
//! This module implements the GridLayoutCoordinator which manages multi-pass layout
//! for CSS Grid Level 2 subgrid bidirectional sizing and CSS Grid Level 3 masonry.

use std::collections::HashMap;

use taffy::prelude::*;

use super::super::grid_errors::{SubgridError, SubgridResult};
use super::layout_states::*;
use super::types::*;

/// Core CSS Grid layout coordination system implementing multi-pass layout
/// for CSS Grid Level 2 subgrid bidirectional sizing and CSS Grid Level 3 masonry
pub struct GridLayoutCoordinator {
    /// Active layout passes for each grid container
    pub layout_passes: HashMap<NodeId, LayoutPassState>,
    /// Subgrid coordination state
    pub subgrid_states: HashMap<NodeId, SubgridLayoutState>,
    /// Auto-placement algorithm state with CSS order processing
    pub auto_placement_states: HashMap<NodeId, AutoPlacementState>,
    /// Intrinsic sizing coordination state
    pub intrinsic_sizing_states: HashMap<NodeId, IntrinsicSizingState>,
    /// CSS Grid Level 3 masonry layout state
    pub masonry_states: HashMap<NodeId, MasonryLayoutState>,
}

impl GridLayoutCoordinator {
    /// Create new coordinator instance
    pub fn new() -> Self {
        Self {
            layout_passes: HashMap::new(),
            subgrid_states: HashMap::new(),
            auto_placement_states: HashMap::new(),
            intrinsic_sizing_states: HashMap::new(),
            masonry_states: HashMap::new(),
        }
    }

    /// Initialize layout pass for a grid container
    pub fn initialize_layout_pass(
        &mut self,
        grid_id: NodeId,
        items: Vec<NodeId>,
    ) -> SubgridResult<()> {
        let ordered_items = self.sort_items_by_css_order(items)?;

        let initial_state = LayoutPassState::InitialPlacement {
            items_by_order: ordered_items.iter().map(|item| item.node_id).collect(),
            auto_cursor: AutoPlacementCursor {
                row: 1,
                column: 1,
                direction: AutoFlowDirection::Row,
            },
            determined_track_counts: DeterminedTrackCounts {
                explicit_rows: 0,
                explicit_columns: 0,
                implicit_rows: 0,
                implicit_columns: 0,
            },
        };

        self.layout_passes.insert(grid_id, initial_state);
        Ok(())
    }

    /// Sort grid items by CSS order property for auto-placement
    fn sort_items_by_css_order(&self, items: Vec<NodeId>) -> SubgridResult<Vec<OrderedGridItem>> {
        let mut ordered_items = Vec::new();

        for item_id in items {
            // In a full implementation, extract CSS order from computed styles
            let css_order = 0; // Simplified default

            let ordered_item = OrderedGridItem {
                node_id: item_id,
                css_order,
                placement: GridItemPlacement {
                    row: GridLineRange {
                        start: GridLine::Auto,
                        end: GridLine::Auto,
                    },
                    column: GridLineRange {
                        start: GridLine::Auto,
                        end: GridLine::Auto,
                    },
                },
            };

            ordered_items.push(ordered_item);
        }

        // Sort by CSS order value
        ordered_items.sort_by_key(|item| item.css_order);

        Ok(ordered_items)
    }

    /// Process auto-placement for items with CSS order
    pub fn process_auto_placement(&mut self, grid_id: NodeId) -> SubgridResult<()> {
        let layout_state =
            self.layout_passes
                .get(&grid_id)
                .ok_or_else(|| SubgridError::CoordinationFailed {
                    details: "Auto-placement failed: No layout state found".to_string(),
                })?;

        if let LayoutPassState::InitialPlacement { items_by_order, .. } = layout_state {
            let items = items_by_order.clone();

            // Initialize auto-placement state
            self.auto_placement_states.insert(
                grid_id,
                AutoPlacementState {
                    cursor: AutoPlacementCursor {
                        row: 1,
                        column: 1,
                        direction: AutoFlowDirection::Row,
                    },
                    pending_items: items,
                    placed_items: Vec::new(),
                    track_occupancy: TrackOccupancyState {
                        occupied_cells: HashMap::new(),
                        row_occupancy: Vec::new(),
                        column_occupancy: Vec::new(),
                    },
                },
            );

            self.execute_auto_placement_algorithm(grid_id)?;
        }

        Ok(())
    }

    /// Execute the CSS Grid auto-placement algorithm
    fn execute_auto_placement_algorithm(&mut self, grid_id: NodeId) -> SubgridResult<()> {
        // Collect pending items first to avoid borrowing conflicts
        let mut pending_items = {
            let auto_state = self
                .auto_placement_states
                .get_mut(&grid_id)
                .ok_or_else(|| SubgridError::CoordinationFailed {
                    details: "Auto-placement algorithm failed: No auto-placement state found"
                        .to_string(),
                })?;
            std::mem::take(&mut auto_state.pending_items)
        };

        // Process items in CSS order
        while let Some(item_id) = pending_items.pop() {
            let placement = self.find_auto_placement_position(grid_id, item_id)?;
            self.place_item_at_position(grid_id, item_id, placement)?;

            if let Some(state) = self.auto_placement_states.get_mut(&grid_id) {
                state.placed_items.push(item_id);
            }
        }

        Ok(())
    }

    /// Find auto-placement position for an item
    fn find_auto_placement_position(
        &mut self,
        grid_id: NodeId,
        item_id: NodeId,
    ) -> SubgridResult<ItemPlacement> {
        // Simplified auto-placement - in full implementation this would
        // follow the CSS Grid auto-placement algorithm exactly
        let auto_state = self.auto_placement_states.get(&grid_id).ok_or_else(|| {
            SubgridError::CoordinationFailed {
                details: "Find auto-placement position failed: No auto-placement state found"
                    .to_string(),
            }
        })?;

        Ok(ItemPlacement {
            item_id,
            parent_grid_row_start: auto_state.cursor.row,
            parent_grid_row_end: auto_state.cursor.row + 1,
            parent_grid_column_start: auto_state.cursor.column,
            parent_grid_column_end: auto_state.cursor.column + 1,
        })
    }

    /// Place item at the specified position
    fn place_item_at_position(
        &mut self,
        grid_id: NodeId,
        item_id: NodeId,
        placement: ItemPlacement,
    ) -> SubgridResult<()> {
        // Update track occupancy
        if let Some(auto_state) = self.auto_placement_states.get_mut(&grid_id) {
            for row in placement.parent_grid_row_start..placement.parent_grid_row_end {
                for col in placement.parent_grid_column_start..placement.parent_grid_column_end {
                    auto_state
                        .track_occupancy
                        .occupied_cells
                        .insert((row, col), item_id);
                }
            }

            // Advance cursor for next item
            auto_state.cursor.column += 1;
        }

        Ok(())
    }

    /// Initialize intrinsic sizing coordination
    pub fn initialize_intrinsic_sizing(&mut self, grid_id: NodeId) -> SubgridResult<()> {
        let intrinsic_state = IntrinsicSizingState {
            row_sizing: AxisSizingState {
                track_sizes: Vec::new(),
                sizing_constraints: Vec::new(),
                flexible_tracks: Vec::new(),
            },
            column_sizing: AxisSizingState {
                track_sizes: Vec::new(),
                sizing_constraints: Vec::new(),
                flexible_tracks: Vec::new(),
            },
            cross_axis_deps: Vec::new(),
            coordination_pass: 0,
            previous_row_sizes: None,
            previous_column_sizes: None,
        };

        self.intrinsic_sizing_states
            .insert(grid_id, intrinsic_state);
        Ok(())
    }

    /// Execute intrinsic sizing coordination
    pub fn execute_intrinsic_sizing(&mut self, grid_id: NodeId) -> SubgridResult<()> {
        const MAX_COORDINATION_PASSES: usize = 5;

        let mut coordination_complete = false;
        let mut pass_count = 0;

        while !coordination_complete && pass_count < MAX_COORDINATION_PASSES {
            coordination_complete = self.execute_intrinsic_sizing_pass(grid_id)?;
            pass_count += 1;

            if let Some(state) = self.intrinsic_sizing_states.get_mut(&grid_id) {
                state.coordination_pass = pass_count;
            }
        }

        if !coordination_complete {
            return Err(SubgridError::CoordinationFailed {
                details:
                    "Intrinsic sizing failed: Failed to converge after maximum coordination passes"
                        .to_string(),
            });
        }

        Ok(())
    }

    /// Execute one pass of intrinsic sizing
    fn execute_intrinsic_sizing_pass(&mut self, grid_id: NodeId) -> SubgridResult<bool> {
        // Phase 1: Calculate row axis sizes
        self.calculate_axis_intrinsic_sizes(grid_id, AbstractAxis::Block)?;

        // Phase 2: Calculate column axis sizes
        self.calculate_axis_intrinsic_sizes(grid_id, AbstractAxis::Inline)?;

        // Phase 3: Check for convergence FIRST (compares previous from last pass vs current)
        let converged = self.check_intrinsic_sizing_convergence(grid_id)?;

        // Phase 4: Store current sizes as previous for NEXT pass comparison
        if let Some(state) = self.intrinsic_sizing_states.get_mut(&grid_id) {
            state.previous_row_sizes = Some(state.row_sizing.track_sizes.clone());
            state.previous_column_sizes = Some(state.column_sizing.track_sizes.clone());
        }

        Ok(converged)
    }

    /// Calculate intrinsic sizes for one axis
    fn calculate_axis_intrinsic_sizes(
        &mut self,
        grid_id: NodeId,
        axis: AbstractAxis,
    ) -> SubgridResult<()> {
        // Simplified intrinsic sizing calculation
        // In full implementation this would follow CSS Grid track sizing algorithm

        if let Some(state) = self.intrinsic_sizing_states.get_mut(&grid_id) {
            let axis_state = match axis {
                AbstractAxis::Block => &mut state.row_sizing,
                AbstractAxis::Inline => &mut state.column_sizing,
            };

            // Initialize track sizes if empty
            if axis_state.track_sizes.is_empty() {
                axis_state.track_sizes = vec![100.0; 3]; // Default 3 tracks of 100px
            }
        }

        Ok(())
    }

    /// Check if intrinsic sizing has converged by comparing track sizes between passes
    fn check_intrinsic_sizing_convergence(&self, grid_id: NodeId) -> SubgridResult<bool> {
        const CONVERGENCE_TOLERANCE: f32 = 0.1;

        if let Some(state) = self.intrinsic_sizing_states.get(&grid_id) {
            // First pass always returns false (not converged yet)
            if state.coordination_pass == 0 {
                return Ok(false);
            }

            // Check if we have previous sizes to compare against
            let row_converged = match &state.previous_row_sizes {
                Some(prev_sizes) => {
                    Self::track_sizes_converged(prev_sizes, &state.row_sizing.track_sizes, CONVERGENCE_TOLERANCE)
                }
                None => false, // No previous data means not converged
            };

            let column_converged = match &state.previous_column_sizes {
                Some(prev_sizes) => {
                    Self::track_sizes_converged(prev_sizes, &state.column_sizing.track_sizes, CONVERGENCE_TOLERANCE)
                }
                None => false,
            };

            // Converged when both axes are stable
            Ok(row_converged && column_converged)
        } else {
            Ok(false)
        }
    }

    /// Helper function for comparing track sizes with tolerance
    #[inline]
    fn track_sizes_converged(prev_sizes: &[f32], current_sizes: &[f32], tolerance: f32) -> bool {
        if prev_sizes.len() != current_sizes.len() {
            return false;
        }

        prev_sizes
            .iter()
            .zip(current_sizes.iter())
            .all(|(prev, curr)| (prev - curr).abs() < tolerance)
    }

    /// Initialize masonry layout state
    pub fn initialize_masonry_layout(&mut self, grid_id: NodeId) -> SubgridResult<()> {
        let masonry_state = MasonryLayoutState {
            masonry_axis: AbstractAxis::Block,
            grid_axis: AbstractAxis::Inline,
            track_positions: Vec::new(),
            item_sizes: HashMap::new(),
            packing_state: MasonryPackingState {
                next_item_index: 0,
                current_masonry_position: 0.0,
                packing_strategy: MasonryPackingStrategy::Shortest,
            },
        };

        self.masonry_states.insert(grid_id, masonry_state);
        Ok(())
    }

    /// Finalize layout with resolved track sizes
    pub fn finalize_layout(&mut self, grid_id: NodeId) -> SubgridResult<()> {
        let resolved_tracks = ResolvedTrackSizes {
            row_sizes: vec![100.0; 3],    // Simplified
            column_sizes: vec![100.0; 3], // Simplified
            row_positions: vec![0.0, 100.0, 200.0, 300.0],
            column_positions: vec![0.0, 100.0, 200.0, 300.0],
        };

        let final_state = LayoutPassState::FinalLayout {
            resolved_tracks,
            item_placements: HashMap::new(),
            masonry_coordination: None,
        };

        self.layout_passes.insert(grid_id, final_state);
        Ok(())
    }
}

impl Default for GridLayoutCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Grid item with CSS order for auto-placement
#[derive(Debug, Clone)]
pub struct OrderedGridItem {
    /// Node ID of the grid item
    pub node_id: NodeId,
    /// CSS order value for placement ordering
    pub css_order: i32,
    /// Item's grid placement (may be auto)
    pub placement: GridItemPlacement,
}

/// Grid item placement specification
#[derive(Debug, Clone)]
pub struct GridItemPlacement {
    /// Row placement
    pub row: GridLineRange,
    /// Column placement
    pub column: GridLineRange,
}

/// Grid line range (start/end)
#[derive(Debug, Clone)]
pub struct GridLineRange {
    /// Start line
    pub start: GridLine,
    /// End line
    pub end: GridLine,
}

/// Grid line specification
#[derive(Debug, Clone)]
pub enum GridLine {
    /// Auto placement
    Auto,
    /// Specific line number
    Line(i32),
    /// Named line
    Named(String),
    /// Span specification
    Span(u32),
}
