//! Core coordination methods for CSS Grid Multi-Pass Layout System

use taffy::{NodeId, geometry::AbstractAxis, TraversePartialTree};

use crate::BaseDocument;
use super::super::grid_context::ParentGridContext;
use super::super::grid_errors::GridPreprocessingError;
use super::super::intrinsic_sizing::calculate_item_intrinsic_size_for_masonry;
use super::placement_types::*;
use super::types::*;

impl GridLayoutCoordinator {
    /// Setup track inheritance for subgrid (Pass 1)
    pub fn setup_track_inheritance<Tree>(
        &mut self,
        subgrid_id: NodeId,
        parent_context: &ParentGridContext,
        tree: &Tree,
    ) -> Result<(), GridPreprocessingError>
    where
        Tree: taffy::LayoutGridContainer,
    {
        // 1. Determine subgrid span in parent
        let subgrid_span = self.determine_subgrid_span(subgrid_id, parent_context, tree)?;

        // 2. Extract corresponding parent tracks (ACTUAL TRACK DEFINITIONS)
        let inherited_tracks = self.extract_parent_tracks(&subgrid_span, parent_context)?;

        // 3. Replace subgrid's grid-template-* properties
        self.replace_grid_template_properties(subgrid_id, &inherited_tracks)?;

        // 4. Setup line name mapping (parent + local names)
        let line_name_mapping = self.setup_line_name_mapping(subgrid_id, parent_context, tree)?;

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
        tree: &BaseDocument,
        subgrid_id: NodeId,
        inputs: &taffy::tree::LayoutInput,
    ) -> Result<Vec<TrackSizeContribution>, GridPreprocessingError> {
        // 1. Measure content intrinsic sizes for all subgrid items
        let content_sizes = self.measure_content_intrinsic_sizes(tree, subgrid_id, inputs)?;

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
        tree: &BaseDocument,
        subgrid_id: NodeId,
        inputs: &taffy::tree::LayoutInput,
    ) -> Result<(), GridPreprocessingError> {
        // 1. Collect size contributions from subgrid items
        let size_contributions = self.collect_subgrid_size_contributions(tree, subgrid_id, inputs)?;

        // 2. Map contributions to parent grid tracks
        let mapped_contributions =
            self.map_contributions_to_parent_tracks(subgrid_id, size_contributions)?;

        // 3. Update parent grid track sizing
        let track_changes = self.update_parent_track_sizing(tree, subgrid_id, mapped_contributions)?;

        // 4. Trigger parent grid re-layout if track sizes changed
        if track_changes {
            self.trigger_parent_recompute(subgrid_id)?;
        }

        Ok(())
    }
    
    /// Execute bidirectional sizing loop with convergence
    pub fn execute_bidirectional_sizing_loop(
        &mut self,
        tree: &BaseDocument,
        subgrid_id: NodeId,
        parent_context: &ParentGridContext,
        inputs: &taffy::tree::LayoutInput,
    ) -> Result<(), GridPreprocessingError> {
        const MAX_PASSES: usize = 5;
        const TOLERANCE: f32 = 0.1;
        
        let mut pass = 0;
        let mut converged = false;
        
        // Initialize intrinsic sizing state if not present
        if !self.intrinsic_sizing_states.contains_key(&subgrid_id) {
            self.intrinsic_sizing_states.insert(subgrid_id, IntrinsicSizingState {
                content_contributions: std::collections::HashMap::new(),
                track_size_requirements: Vec::new(),
                sizing_pass_state: SizingPassState::default(),
            });
        }
        
        while !converged && pass < MAX_PASSES {
            // PASS A: Inherit parent tracks to subgrid
            self.setup_track_inheritance(subgrid_id, parent_context, tree)?;
            
            // PASS B: Measure subgrid items
            let _intrinsic_contributions = self.measure_content_intrinsic_sizes(tree, subgrid_id, inputs)?;
            
            // PASS C: Collect contributions to parent
            let track_contributions = self.collect_subgrid_size_contributions(tree, subgrid_id, inputs)?;
            
            // PASS D: Update parent track sizing
            let track_changes = self.update_parent_track_sizing(tree, subgrid_id, track_contributions)?;
            
            // PASS E: Check convergence
            converged = !track_changes || self.check_size_convergence(subgrid_id, TOLERANCE)?;
            
            pass += 1;
        }
        
        if !converged {
            #[cfg(feature = "tracing")]
            tracing::warn!("Subgrid bidirectional sizing did not converge after {} passes", MAX_PASSES);
        }
        
        Ok(())
    }
    
    /// Check if track sizes have converged
    fn check_size_convergence(
        &self,
        subgrid_id: NodeId,
        _tolerance: f32,
    ) -> Result<bool, GridPreprocessingError> {
        let state = self.intrinsic_sizing_states.get(&subgrid_id)
            .ok_or_else(|| GridPreprocessingError::preprocessing_failed(
                "convergence_check",
                subgrid_id.into(),
                "Intrinsic sizing state not found"
            ))?;
        
        // For now, we check if we have any track size requirements
        // In full implementation, this would compare with previous_sizes
        let has_requirements = !state.track_size_requirements.is_empty();
        
        Ok(has_requirements)
    }
    
    /// Helper: Check if track sizes converged
    #[allow(dead_code)]
    fn track_sizes_converged(prev: &[f32], current: &[f32], tolerance: f32) -> bool {
        if prev.len() != current.len() {
            return false;
        }
        prev.iter()
            .zip(current.iter())
            .all(|(p, c)| (p - c).abs() < tolerance)
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
        &mut self,
        tree: &BaseDocument,
        subgrid_id: NodeId,
        inputs: &taffy::tree::LayoutInput,
    ) -> Result<Vec<TrackSizeContribution>, GridPreprocessingError> {
        let mut contributions = Vec::new();
        
        // Step 1: Get intrinsic sizes from measurement
        let intrinsic_contributions = self.measure_content_intrinsic_sizes(tree, subgrid_id, inputs)?;
        
        // Step 2: Get subgrid state for coordinate mapping
        let _subgrid_state = self.subgrid_states.get(&subgrid_id)
            .ok_or_else(|| GridPreprocessingError::preprocessing_failed(
                "collect_contributions",
                subgrid_id.into(),
                "Subgrid state not found"
            ))?;
        
        // Step 3: Map each item contribution to parent track coordinates
        for intrinsic in intrinsic_contributions {
            for track_index in intrinsic.affected_tracks {
                // Map subgrid track index to parent track index
                let parent_track_index = self.map_subgrid_track_to_parent(
                    subgrid_id, 
                    track_index, 
                    intrinsic.axis
                )?;
                
                contributions.push(TrackSizeContribution {
                    parent_track_index,
                    axis: intrinsic.axis,
                    min_size: intrinsic.min_content_size,
                    max_size: intrinsic.max_content_size,
                    preferred_size: (intrinsic.min_content_size + intrinsic.max_content_size) / 2.0,
                });
            }
        }
        
        // Step 4: Store in state for future passes
        if let Some(state) = self.subgrid_states.get_mut(&subgrid_id) {
            state.size_contributions = contributions.clone();
        }
        
        Ok(contributions)
    }
    
    /// Map subgrid track index to parent track index
    fn map_subgrid_track_to_parent(
        &self,
        _subgrid_id: NodeId,
        track_index: usize,
        _axis: super::super::grid_context::GridAxis,
    ) -> Result<usize, GridPreprocessingError> {
        // For now, direct mapping - will be enhanced with proper coordinate transformation
        Ok(track_index)
    }

    /// Enhanced intrinsic sizing with content measurement
    pub fn measure_content_intrinsic_sizes(
        &self,
        tree: &BaseDocument,
        subgrid_id: NodeId,
        inputs: &taffy::tree::LayoutInput,
    ) -> Result<Vec<IntrinsicSizeContribution>, GridPreprocessingError> {
        let mut contributions = Vec::new();
        
        // Get all items in the subgrid
        let child_count = tree.child_count(subgrid_id);
        
        for i in 0..child_count {
            let item_id = tree.get_child_id(subgrid_id, i);
            
            // USE EXISTING MEASUREMENT INFRASTRUCTURE
            let min_content_size = calculate_item_intrinsic_size_for_masonry(
                tree,
                item_id,
                &taffy::tree::LayoutInput {
                    available_space: taffy::Size {
                        width: taffy::AvailableSpace::MinContent,
                        height: taffy::AvailableSpace::MinContent,
                    },
                    ..inputs.clone()
                },
                AbstractAxis::Block,
            )?;
            
            let max_content_size = calculate_item_intrinsic_size_for_masonry(
                tree,
                item_id,
                &taffy::tree::LayoutInput {
                    available_space: taffy::Size {
                        width: taffy::AvailableSpace::MaxContent,
                        height: taffy::AvailableSpace::MaxContent,
                    },
                    ..inputs.clone()
                },
                AbstractAxis::Block,
            )?;
            
            // Determine which tracks this item affects
            let affected_tracks = self.determine_affected_tracks(item_id, tree)?;
            
            contributions.push(IntrinsicSizeContribution {
                node_id: item_id,
                min_content_size: min_content_size.width.max(min_content_size.height),
                max_content_size: max_content_size.width.max(max_content_size.height),
                affected_tracks,
                axis: super::super::grid_context::GridAxis::Row,
            });
        }
        
        Ok(contributions)
    }
    
    /// Determine which tracks an item affects
    fn determine_affected_tracks(
        &self,
        _item_id: NodeId,
        _tree: &BaseDocument,
    ) -> Result<Vec<usize>, GridPreprocessingError> {
        // For now, return first track - will be enhanced with proper grid placement analysis
        Ok(vec![0])
    }

    /// Compute min-content size using actual measurement
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
