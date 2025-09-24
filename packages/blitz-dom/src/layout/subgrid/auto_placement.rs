//! Auto-placement algorithms for CSS Grid subgrids
//!
//! This module implements the CSS Grid auto-placement algorithm for subgrids,
//! including cursor tracking, availability checking, and placement coordination.

use taffy::prelude::*;

use super::super::grid_errors::SubgridResult;
use super::types::*;

/// Auto-placement cursor for tracking position in subgrid
#[derive(Debug, Clone)]
pub struct AutoPlacementCursor {
    /// Current row position
    pub current_row: usize,
    /// Current column position
    pub current_column: usize,
    /// Maximum rows available
    pub max_rows: usize,
    /// Maximum columns available
    pub max_columns: usize,
    /// Flow direction for auto-placement
    pub flow_direction: FlowDirection,
    /// Whether to use dense packing
    pub dense_packing: bool,
}

impl AutoPlacementCursor {
    pub fn new(inheritance_data: &SubgridTrackInheritance) -> Self {
        Self {
            current_row: 0,
            current_column: 0,
            max_rows: inheritance_data.inherited_row_tracks.len(),
            max_columns: inheritance_data.inherited_column_tracks.len(),
            flow_direction: FlowDirection::Row, // CSS default
            dense_packing: false,               // CSS default
        }
    }

    /// Advance cursor to next position per CSS Grid auto-placement algorithm
    pub fn advance_to_next_position(&mut self, inheritance_data: &SubgridTrackInheritance) -> bool {
        match self.flow_direction {
            FlowDirection::Row => {
                self.current_column += 1;
                if self.current_column >= inheritance_data.inherited_column_tracks.len() {
                    self.current_column = 0;
                    self.current_row += 1;
                    if self.current_row >= inheritance_data.inherited_row_tracks.len() {
                        return false; // Exhausted grid
                    }
                }
            }
            FlowDirection::Column => {
                self.current_row += 1;
                if self.current_row >= inheritance_data.inherited_row_tracks.len() {
                    self.current_row = 0;
                    self.current_column += 1;
                    if self.current_column >= inheritance_data.inherited_column_tracks.len() {
                        return false; // Exhausted grid
                    }
                }
            }
        }
        true
    }

    pub fn current_position(&self) -> GridPosition {
        GridPosition {
            row_start: self.current_row,
            column_start: self.current_column,
        }
    }

    pub fn advance_past_placed_item(&mut self, _placement: &SubgridItemPlacement) {
        // Simplified advancement - real implementation would consider item span
        self.current_column += 1;
        if self.current_column >= self.max_columns {
            self.current_column = 0;
            self.current_row += 1;
        }
    }
}

/// Grid position for auto-placement calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition {
    /// Row start position
    pub row_start: usize,
    /// Column start position
    pub column_start: usize,
}

/// Flow direction for auto-placement
#[derive(Debug, Clone, Copy)]
pub enum FlowDirection {
    Row,
    Column,
}

/// Item span information for placement calculations
#[derive(Debug, Clone)]
pub struct ItemSpan {
    pub row_span: usize,
    pub column_span: usize,
}

/// Subgrid item representation for placement algorithm
#[derive(Debug, Clone)]
pub struct SubgridItem {
    pub node_id: NodeId,
    pub explicit_placement: Option<GridPlacement>,
    pub item_type: GridItemType,
}

impl SubgridItem {
    pub fn has_explicit_placement(&self) -> bool {
        self.explicit_placement.is_some()
    }
}

/// Grid item type classification
#[derive(Debug, Clone)]
pub enum GridItemType {
    GridItem,
    SubgridItem,
    NestedSubgrid,
}

/// Grid placement specification
#[derive(Debug, Clone)]
pub struct GridPlacement {
    pub row_start: usize,
    pub row_end: usize,
    pub column_start: usize,
    pub column_end: usize,
}

/// Subgrid item placement result with coordinate mapping
///
/// Maps between subgrid-local coordinates and parent grid coordinates
/// using existing coordinate transformation infrastructure.
#[derive(Debug, Clone)]
pub struct SubgridItemPlacement {
    pub item_id: NodeId,

    // Parent grid coordinates (for final layout calculation)
    pub parent_grid_row_start: usize,
    pub parent_grid_row_end: usize,
    pub parent_grid_column_start: usize,
    pub parent_grid_column_end: usize,

    // Subgrid-local coordinates (for CSS compliance)
    pub subgrid_local_row_start: usize,
    pub subgrid_local_row_end: usize,
    pub subgrid_local_column_start: usize,
    pub subgrid_local_column_end: usize,

    // Placement method for debugging/validation
    pub placement_method: PlacementMethod,
}

/// Placement method for tracking how items were placed
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementMethod {
    ExplicitGridRow,
    ExplicitGridColumn,
    ExplicitBoth,
    AutoPlacement { cursor_position: GridPosition },
}

/// Track availability state for inherited parent tracks
///
/// Optimized for subgrid scenarios where we track occupancy within
/// a subset of parent grid tracks rather than the full parent grid.
#[derive(Debug, Clone)]
pub struct TrackAvailability {
    /// Occupied ranges within this track (sparse representation for performance)
    pub occupied_ranges: Vec<OccupiedRange>,
    /// Track size from inheritance data
    pub track_size: f32,
    /// Track index in parent grid space
    pub parent_track_index: usize,
}

impl TrackAvailability {
    pub fn new(parent_track_index: usize) -> Self {
        Self {
            occupied_ranges: Vec::new(),
            track_size: 0.0,
            parent_track_index,
        }
    }

    /// Check if a range is available (no overlaps with occupied ranges)
    pub fn is_range_available(&self, start_pos: f32, end_pos: f32) -> bool {
        for range in &self.occupied_ranges {
            if range.overlaps(start_pos, end_pos) {
                return false;
            }
        }
        true
    }

    /// Mark a range as occupied, merging with adjacent ranges
    pub fn mark_range_occupied(
        &mut self,
        start_pos: f32,
        end_pos: f32,
        occupying_item: NodeId,
        placement_method: PlacementMethod,
    ) -> SubgridResult<()> {
        let new_range = OccupiedRange {
            start_position: start_pos,
            end_position: end_pos,
            occupying_item,
            placement_method,
        };

        // Insert in sorted order using binary search
        let insert_pos = self
            .occupied_ranges
            .binary_search_by(|r| {
                r.start_position
                    .partial_cmp(&start_pos)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|pos| pos);

        self.occupied_ranges.insert(insert_pos, new_range);
        self.merge_adjacent_ranges();

        Ok(())
    }

    /// Find next available position after given start
    pub fn get_next_available_position(&self, start_pos: f32) -> f32 {
        for range in &self.occupied_ranges {
            if range.start_position <= start_pos && start_pos < range.end_position {
                return range.end_position;
            }
        }
        start_pos // Already available
    }

    /// Set track size from layout computation
    pub fn set_track_size(&mut self, size: f32) {
        self.track_size = size;
    }

    /// Get track size
    pub fn get_track_size(&self) -> f32 {
        self.track_size
    }

    /// Merge overlapping/adjacent ranges for memory efficiency
    fn merge_adjacent_ranges(&mut self) {
        if self.occupied_ranges.len() <= 1 {
            return;
        }

        let mut merged = Vec::new();
        let mut current = self.occupied_ranges[0].clone();

        for range in self.occupied_ranges.iter().skip(1) {
            if current.end_position >= range.start_position {
                // Overlapping or adjacent - merge
                current.end_position = current.end_position.max(range.end_position);
            } else {
                // Gap - push current and start new
                merged.push(current);
                current = range.clone();
            }
        }
        merged.push(current);

        self.occupied_ranges = merged;
    }
}

/// Occupied range within a track
#[derive(Debug, Clone)]
pub struct OccupiedRange {
    pub start_position: f32,
    pub end_position: f32,
    pub occupying_item: NodeId,
    pub placement_method: PlacementMethod,
}

impl OccupiedRange {
    /// Check if this range overlaps with given range
    pub fn overlaps(&self, start: f32, end: f32) -> bool {
        !(end <= self.start_position || start >= self.end_position)
    }

    /// Check if this range contains a specific position
    pub fn contains(&self, position: f32) -> bool {
        position >= self.start_position && position < self.end_position
    }

    /// Check if this range can be merged with another
    pub fn can_merge_with(&self, other: &OccupiedRange) -> bool {
        self.end_position >= other.start_position && other.end_position >= self.start_position
    }

    /// Get the occupying item
    pub fn get_occupying_item(&self) -> NodeId {
        self.occupying_item
    }

    /// Get the placement method
    pub fn get_placement_method(&self) -> &PlacementMethod {
        &self.placement_method
    }
}

/// Subgrid placement state coordinating with parent grid
#[derive(Debug)]
pub struct SubgridPlacementState {
    /// Current auto-placement cursor position
    pub cursor_row: usize,
    pub cursor_column: usize,
    /// Flow direction for auto-placement
    pub flow_direction: FlowDirection,
    /// Dense packing mode
    pub dense_packing: bool,
}

impl SubgridPlacementState {
    pub fn new() -> Self {
        Self {
            cursor_row: 0,
            cursor_column: 0,
            flow_direction: FlowDirection::Row,
            dense_packing: false,
        }
    }
}

impl Default for SubgridPlacementState {
    fn default() -> Self {
        Self::new()
    }
}
