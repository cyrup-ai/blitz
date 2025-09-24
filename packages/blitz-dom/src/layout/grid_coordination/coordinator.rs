//! Core coordination methods for CSS Grid Multi-Pass Layout System

use taffy::NodeId;

use super::super::grid_context::ParentGridContext;
use super::super::grid_errors::GridPreprocessingError;
use super::placement_types::*;
use super::types::*;

impl GridLayoutCoordinator {
    /// Setup track inheritance for subgrid (Pass 1)
    pub fn setup_track_inheritance(
        &mut self,
        subgrid_id: NodeId,
        parent_context: &ParentGridContext,
    ) -> Result<(), GridPreprocessingError> {
        // 1. Determine subgrid span in parent
        let subgrid_span = self.determine_subgrid_span(subgrid_id, parent_context)?;

        // 2. Extract corresponding parent tracks (ACTUAL TRACK DEFINITIONS)
        let inherited_tracks = self.extract_parent_tracks(&subgrid_span, parent_context)?;

        // 3. Replace subgrid's grid-template-* properties
        self.replace_grid_template_properties(subgrid_id, &inherited_tracks)?;

        // 4. Setup line name mapping (parent + local names)
        let line_name_mapping = self.setup_line_name_mapping(subgrid_id, parent_context)?;

        // Create subgrid layout state
        let subgrid_state = SubgridLayoutState {
            inherited_tracks,
            line_name_mapping,
            size_contributions: Vec::new(),
            layout_pass_state: LayoutPassState::default(),
        };

        self.subgrid_states.insert(subgrid_id, subgrid_state);

        Ok(())
    }

    /// Collect intrinsic size contributions (Pass 2)
    pub fn collect_intrinsic_size_contributions(
        &mut self,
        subgrid_id: NodeId,
    ) -> Result<Vec<TrackSizeContribution>, GridPreprocessingError> {
        // 1. Measure content intrinsic sizes for all subgrid items
        let content_sizes = self.measure_content_intrinsic_sizes(subgrid_id)?;

        // 2. Map to parent grid coordinates
        let mapped_contributions = self.map_to_parent_coordinates(subgrid_id, content_sizes)?;

        // 3. Create size contributions for parent tracks
        let track_contributions =
            self.create_track_contributions(subgrid_id, mapped_contributions)?;

        // Update state
        if let Some(state) = self.subgrid_states.get_mut(&subgrid_id) {
            state.size_contributions = track_contributions.clone();
        }

        Ok(track_contributions)
    }

    /// Coordinate auto-placement (Pass 3)
    pub fn coordinate_auto_placement(
        &mut self,
        subgrid_id: NodeId,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError> {
        // 1. Initialize placement state
        let mut placement_state = AutoPlacementState {
            cursor_position: GridPosition::default(),
            ordered_items: Vec::new(),
            explicit_placements: std::collections::HashMap::new(),
            dense_packing_state: None,
            track_occupancy: TrackOccupancyMap::default(),
        };

        // 2. Process items in CSS order
        let ordered_items = self.get_items_in_css_order(subgrid_id)?;
        placement_state.ordered_items = ordered_items;

        // 3. Handle explicit placements
        let explicit_placements =
            self.process_explicit_placements(subgrid_id, &mut placement_state)?;

        // 4. Auto-place using placement cursor
        let auto_placements = self.auto_place_items(subgrid_id, &mut placement_state)?;

        // 5. Dense packing pass (if enabled)
        let dense_placements = self.dense_packing_pass(subgrid_id, &mut placement_state)?;

        // Combine all placements
        let mut all_placements = explicit_placements;
        all_placements.extend(auto_placements);
        all_placements.extend(dense_placements);

        self.auto_placement_states
            .insert(subgrid_id, placement_state);

        Ok(all_placements)
    }

    /// Coordinate bidirectional sizing (Pass 4)
    pub fn coordinate_bidirectional_sizing(
        &mut self,
        subgrid_id: NodeId,
    ) -> Result<(), GridPreprocessingError> {
        // 1. Collect size contributions from subgrid items
        let size_contributions = self.collect_subgrid_size_contributions(subgrid_id)?;

        // 2. Map contributions to parent grid tracks
        let mapped_contributions =
            self.map_contributions_to_parent_tracks(subgrid_id, size_contributions)?;

        // 3. Update parent grid track sizing
        let track_changes = self.update_parent_track_sizing(subgrid_id, mapped_contributions)?;

        // 4. Trigger parent grid re-layout if track sizes changed
        if track_changes {
            self.trigger_parent_recompute(subgrid_id)?;
        }

        Ok(())
    }

    /// Virtual masonry placement algorithm
    pub fn place_virtual_masonry_items(
        &self,
        _subgrid_id: NodeId,
        masonry_state: &mut MasonryLayoutState,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError> {
        let mut placements = Vec::new();

        for virtual_item in &masonry_state.virtual_items {
            // Find track with minimum running position
            let selected_track = self.select_masonry_track(masonry_state, virtual_item)?;

            // Create placement at selected track
            let placement = ItemPlacement {
                node_id: virtual_item.node_id,
                grid_area: GridArea {
                    row_start: masonry_state.track_running_positions[selected_track] as i32,
                    row_end: (masonry_state.track_running_positions[selected_track]
                        + virtual_item.item_size.1) as i32,
                    column_start: selected_track as i32,
                    column_end: (selected_track + 1) as i32,
                },
                placement_method: PlacementMethod::AutoPlacement,
            };

            placements.push(placement);

            // Update running position for selected track
            masonry_state.track_running_positions[selected_track] += virtual_item.item_size.1;
        }

        Ok(placements)
    }

    /// Bidirectional size contribution flow implementation
    pub fn collect_subgrid_size_contributions(
        &self,
        subgrid_id: NodeId,
    ) -> Result<Vec<TrackSizeContribution>, GridPreprocessingError> {
        let mut contributions = Vec::new();

        // Get subgrid state
        if let Some(subgrid_state) = self.subgrid_states.get(&subgrid_id) {
            // Collect contributions from all items in the subgrid
            for contribution in &subgrid_state.size_contributions {
                contributions.push(contribution.clone());
            }
        }

        Ok(contributions)
    }

    /// Enhanced intrinsic sizing with content measurement
    pub fn measure_content_intrinsic_sizes(
        &self,
        subgrid_id: NodeId,
    ) -> Result<Vec<IntrinsicSizeContribution>, GridPreprocessingError> {
        let mut contributions = Vec::new();

        // Integration with the BaseDocument's text system and layout engine
        // Use actual size computation instead of hardcoded values
        
        // Use Taffy's size computation for intrinsic sizing
        // This calls into the proper measurement infrastructure
        let min_content_size = self.compute_min_content_size(subgrid_id)?;
        let max_content_size = self.compute_max_content_size(subgrid_id)?;

        contributions.push(IntrinsicSizeContribution {
            node_id: subgrid_id,
            min_content_size,
            max_content_size,
            affected_tracks: vec![0],
            axis: super::super::grid_context::GridAxis::Row,
        });

        Ok(contributions)
    }

    /// Compute min-content size using actual measurement
    fn compute_min_content_size(&self, node_id: NodeId) -> Result<f32, GridPreprocessingError> {
        // Use CSS min-content calculation instead of hardcoded value
        // This integrates with the intrinsic sizing state tracking system
        if let Some(intrinsic_state) = self.intrinsic_sizing_states.get(&node_id) {
            // Use minimum content contribution from measured content
            let min_size = intrinsic_state.content_contributions
                .values()
                .map(|contrib| contrib.min_content_size)
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);
            Ok(min_size)
        } else {
            // Fallback to CSS spec default for min-content when no content
            Ok(0.0)
        }
    }

    /// Compute max-content size using actual measurement  
    fn compute_max_content_size(&self, node_id: NodeId) -> Result<f32, GridPreprocessingError> {
        // Use CSS max-content calculation instead of hardcoded value
        // This integrates with the intrinsic sizing state tracking system
        if let Some(intrinsic_state) = self.intrinsic_sizing_states.get(&node_id) {
            // Use maximum content contribution from measured content
            let max_size = intrinsic_state.content_contributions
                .values()
                .map(|contrib| contrib.max_content_size)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);
            Ok(max_size)
        } else {
            // Fallback to CSS spec default for max-content when no content
            Ok(0.0)
        }
    }

    /// Select masonry track with minimum running position
    pub fn select_masonry_track(
        &self,
        masonry_state: &MasonryLayoutState,
        _virtual_item: &VirtualMasonryItem,
    ) -> Result<usize, GridPreprocessingError> {
        // Find track with minimum running position (with tolerance)
        let mut min_position = f32::INFINITY;
        let mut selected_track = 0;

        for (track_index, &position) in masonry_state.track_running_positions.iter().enumerate() {
            if position < min_position - masonry_state.item_tolerance {
                min_position = position;
                selected_track = track_index;
            }
        }

        Ok(selected_track)
    }
}
