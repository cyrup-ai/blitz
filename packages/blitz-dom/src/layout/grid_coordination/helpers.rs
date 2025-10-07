//! Helper methods for CSS Grid Layout Coordination

use taffy::NodeId;

use super::super::grid_context::{GridAxis, ParentGridContext};
use super::super::grid_errors::GridPreprocessingError;
use super::placement_types::*;
use super::track_types::*;
use super::types::*;

impl GridLayoutCoordinator {
    /// Helper: Determine subgrid span in parent by reading actual grid placement from styles
    pub fn determine_subgrid_span<Tree>(
        &self,
        subgrid_id: NodeId,
        parent_context: &ParentGridContext,
        tree: &Tree,
    ) -> Result<GridArea, GridPreprocessingError>
    where
        Tree: taffy::LayoutGridContainer,
    {
        use taffy::GridItemStyle;

        // Get the subgrid's grid placement from its style
        let subgrid_style = tree.get_grid_child_style(subgrid_id);
        let grid_row = subgrid_style.grid_row();
        let grid_column = subgrid_style.grid_column();

        // Convert placements to origin-zero coordinates
        // Account for explicit track counts from parent
        let row_placement = grid_row.into_origin_zero_ignoring_named(
            parent_context.row_track_count as u16
        );
        let column_placement = grid_column.into_origin_zero_ignoring_named(
            parent_context.column_track_count as u16
        );

        // Resolve definite placements or handle auto-placement
        // Note: OriginZeroLine is pub(crate) in taffy, so we handle both cases and convert to GridArea
        let grid_area = if row_placement.is_definite() && column_placement.is_definite() {
            let row_range = row_placement.resolve_definite_grid_lines();
            let column_range = column_placement.resolve_definite_grid_lines();
            
            // Convert from 0-based OriginZeroLine to 1-based GridArea
            GridArea {
                row_start: (row_range.start.0 as i32) + 1,
                row_end: (row_range.end.0 as i32) + 1,
                column_start: (column_range.start.0 as i32) + 1,
                column_end: (column_range.end.0 as i32) + 1,
            }
        } else {
            // Auto placement - use default span covering first few tracks
            // Since we can't construct OriginZeroLine (it's pub(crate)), we construct GridArea directly
            let row_end = if row_placement.is_definite() {
                let row_range = row_placement.resolve_definite_grid_lines();
                (row_range.end.0 as i32) + 1
            } else {
                (parent_context.row_track_count as i32).min(3)
            };
            
            let column_end = if column_placement.is_definite() {
                let column_range = column_placement.resolve_definite_grid_lines();
                (column_range.end.0 as i32) + 1
            } else {
                (parent_context.column_track_count as i32).min(4)
            };
            
            GridArea {
                row_start: 1,
                row_end,
                column_start: 1,
                column_end,
            }
        };

        Ok(grid_area)
    }

    /// Helper: Extract parent tracks for inheritance
    pub fn extract_parent_tracks(
        &self,
        subgrid_span: &GridArea,
        parent_context: &ParentGridContext,
    ) -> Result<InheritedTrackDefinitions, GridPreprocessingError> {
        // Convert grid area lines to track indices (1-based to 0-based)
        let row_start = (subgrid_span.row_start - 1).max(0) as usize;
        let row_end = (subgrid_span.row_end - 1).min(parent_context.row_track_count as i32) as usize;
        let col_start = (subgrid_span.column_start - 1).max(0) as usize;
        let col_end = (subgrid_span.column_end - 1).min(parent_context.column_track_count as i32) as usize;

        // Extract actual parent track slices
        let parent_row_slice = parent_context
            .parent_row_tracks
            .get(row_start..row_end)
            .unwrap_or(&[]);
        
        let parent_column_slice = parent_context
            .parent_column_tracks
            .get(col_start..col_end)
            .unwrap_or(&[]);

        // Convert TrackSizingFunction to TrackDefinition using existing infrastructure
        let row_tracks = parent_row_slice
            .iter()
            .map(|track_fn| convert_sizing_function_to_definition(track_fn))
            .collect();

        let column_tracks = parent_column_slice
            .iter()
            .map(|track_fn| convert_sizing_function_to_definition(track_fn))
            .collect();

        // Convert taffy::TrackSizingFunction to our internal TrackSizingFunction
        let row_sizing_functions = parent_row_slice
            .iter()
            .map(|track_fn| convert_taffy_sizing_function_to_internal(track_fn, parent_context.parent_size, GridAxis::Row))
            .collect();

        let column_sizing_functions = parent_column_slice
            .iter()
            .map(|track_fn| convert_taffy_sizing_function_to_internal(track_fn, parent_context.parent_size, GridAxis::Column))
            .collect();

        Ok(InheritedTrackDefinitions {
            row_tracks,
            column_tracks,
            row_sizing_functions,
            column_sizing_functions,
        })
    }

    /// Helper: Replace grid-template-* properties
    /// 
    /// Converts inherited track definitions to Taffy-compatible track sizing functions.
    /// Returns the converted functions for application in the calling context where
    /// BaseDocument access is available.
    /// This implements the CSS Grid Level 2 subgrid property replacement algorithm.
    pub fn replace_grid_template_properties(
        &mut self,
        subgrid_id: NodeId,
        inherited_tracks: &InheritedTrackDefinitions,
    ) -> Result<ReplacedGridTemplates, GridPreprocessingError> {
        // Convert inherited track definitions to taffy-compatible track sizing functions
        let row_track_functions: Result<Vec<taffy::TrackSizingFunction>, GridPreprocessingError> = 
            inherited_tracks.row_sizing_functions.iter()
                .map(|internal_fn| convert_internal_to_taffy_sizing_function(internal_fn))
                .collect();
        
        let column_track_functions: Result<Vec<taffy::TrackSizingFunction>, GridPreprocessingError> = 
            inherited_tracks.column_sizing_functions.iter()
                .map(|internal_fn| convert_internal_to_taffy_sizing_function(internal_fn))
                .collect();
        
        let row_functions = row_track_functions?;
        let column_functions = column_track_functions?;

        #[cfg(feature = "tracing")]
        {
            if !row_functions.is_empty() {
                tracing::debug!(
                    "Prepared subgrid row template replacement for node {:?}: {} inherited tracks",
                    subgrid_id,
                    row_functions.len()
                );
            }

            if !column_functions.is_empty() {
                tracing::debug!(
                    "Prepared subgrid column template replacement for node {:?}: {} inherited tracks",
                    subgrid_id,
                    column_functions.len()
                );
            }
        }

        // Return the converted tracks for application in the calling context
        Ok(ReplacedGridTemplates {
            row_functions,
            column_functions,
        })
    }

    /// Helper: Setup line name mapping
    pub fn setup_line_name_mapping<Tree>(
        &self,
        subgrid_id: NodeId,
        parent_context: &ParentGridContext,
        tree: &Tree,
    ) -> Result<LineNameMap, GridPreprocessingError>
    where
        Tree: taffy::LayoutGridContainer,
    {
        use std::collections::HashMap;
        
        // Extract parent line names for the subgrid span
        let subgrid_span = self.determine_subgrid_span(subgrid_id, parent_context, tree)?;
        
        // Get parent row line names for subgrid span
        let mut parent_line_names = HashMap::new();
        let row_start = (subgrid_span.row_start - 1).max(0) as usize;
        let row_end_inclusive = (subgrid_span.row_end).min(parent_context.parent_row_line_names.len() as i32) as usize;
        
        for (local_index, parent_index) in (row_start..=row_end_inclusive).enumerate() {
            if let Some(line_names) = parent_context.parent_row_line_names.get(parent_index) {
                for name in line_names {
                    parent_line_names.insert(name.clone(), vec![local_index as i32 + 1]);
                }
            }
        }
        
        // Get parent column line names for subgrid span  
        let col_start = (subgrid_span.column_start - 1).max(0) as usize;
        let col_end_inclusive = (subgrid_span.column_end).min(parent_context.parent_column_line_names.len() as i32) as usize;
        
        for (local_index, parent_index) in (col_start..=col_end_inclusive).enumerate() {
            if let Some(line_names) = parent_context.parent_column_line_names.get(parent_index) {
                for name in line_names {
                    parent_line_names.entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push(local_index as i32 + 1);
                }
            }
        }
        
        // Create combined mapping (parent + local names)
        let combined_mapping = parent_line_names.clone();
        
        Ok(LineNameMap {
            parent_line_names,
            local_line_names: HashMap::new(), // Would be populated from CSS parsing
            combined_mapping,
        })
    }

    /// Helper: Map to parent coordinates
    pub fn map_to_parent_coordinates(
        &self,
        _subgrid_id: NodeId,
        content_sizes: Vec<IntrinsicSizeContribution>,
    ) -> Result<Vec<IntrinsicSizeContribution>, GridPreprocessingError> {
        Ok(content_sizes)
    }

    /// Helper: Create track contributions
    pub fn create_track_contributions(
        &self,
        _subgrid_id: NodeId,
        _mapped_contributions: Vec<IntrinsicSizeContribution>,
    ) -> Result<Vec<TrackSizeContribution>, GridPreprocessingError> {
        Ok(Vec::new())
    }

    /// Helper: Get items in CSS order
    pub fn get_items_in_css_order<Tree>(
        &self,
        subgrid_id: NodeId,
        tree: &Tree,
    ) -> Result<Vec<(NodeId, i32)>, GridPreprocessingError>
    where
        Tree: taffy::TraversePartialTree + std::any::Any,
    {
        use crate::BaseDocument;

        // Get all children of the subgrid
        let child_count = tree.child_count(subgrid_id);
        let mut items_with_order = Vec::with_capacity(child_count);

        // Downcast to BaseDocument to access order property
        if let Some(base_doc) = (tree as &dyn std::any::Any).downcast_ref::<BaseDocument>() {
            // Iterate through children and extract order values
            for i in 0..child_count {
                let child_id = tree.get_child_id(subgrid_id, i);

                // Access node and get order property
                let node = &base_doc.nodes[usize::from(child_id)];
                let order_value = node.order();

                items_with_order.push((child_id, order_value));
            }

            // Stable sort by order value (preserves DOM order for equal values)
            items_with_order.sort_by_key(|(_, order)| *order);

            Ok(items_with_order)
        } else {
            // If not BaseDocument, return items in DOM order with default order value of 0
            for i in 0..child_count {
                let child_id = tree.get_child_id(subgrid_id, i);
                items_with_order.push((child_id, 0));
            }

            Ok(items_with_order)
        }
    }

    /// Helper: Process explicit placements
    pub fn process_explicit_placements<Tree>(
        &self,
        subgrid_id: NodeId,
        placement_state: &mut AutoPlacementState,
        tree: &Tree,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError>
    where
        Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree,
    {
        use taffy::GridItemStyle;
        use taffy::GridPlacement;

        let mut explicit_placements = Vec::new();

        // Get all children of the subgrid
        let child_count = tree.child_count(subgrid_id);

        for i in 0..child_count {
            let child_id = tree.get_child_id(subgrid_id, i);
            let style = tree.get_grid_child_style(child_id);
            let grid_row = style.grid_row();
            let grid_column = style.grid_column();

            // Only process if at least one dimension is explicitly placed
            let has_explicit_row = !matches!(grid_row.start, GridPlacement::Auto) || !matches!(grid_row.end, GridPlacement::Auto);
            let has_explicit_col = !matches!(grid_column.start, GridPlacement::Auto) || !matches!(grid_column.end, GridPlacement::Auto);

            if has_explicit_row || has_explicit_col {
                // Resolve grid lines to actual positions
                let row_placement = self.resolve_grid_line_placement(
                    grid_row,
                    true, // is_row_axis
                    placement_state,
                )?;

                let col_placement = self.resolve_grid_line_placement(
                    grid_column,
                    false, // is_column_axis
                    placement_state,
                )?;

                let placement = ItemPlacement {
                    node_id: child_id,
                    grid_area: GridArea {
                        row_start: row_placement.0,
                        row_end: row_placement.1,
                        column_start: col_placement.0,
                        column_end: col_placement.1,
                    },
                    placement_method: PlacementMethod::Explicit,
                };

                // Mark cells as occupied
                for row in placement.grid_area.row_start..placement.grid_area.row_end {
                    for col in placement.grid_area.column_start..placement.grid_area.column_end {
                        placement_state
                            .track_occupancy
                            .occupied_cells
                            .insert((row, col), child_id);
                    }
                }

                // Store in explicit_placements map for auto-placement to skip
                placement_state.explicit_placements.insert(child_id, placement.grid_area.clone());

                explicit_placements.push(placement);
            }
        }

        Ok(explicit_placements)
    }

    /// Resolve grid line placement to concrete positions
    fn resolve_grid_line_placement<CustomIdent: taffy::CheapCloneStr>(
        &self,
        line: taffy::Line<taffy::GridPlacement<CustomIdent>>,
        is_row: bool,
        state: &AutoPlacementState,
    ) -> Result<(i32, i32), GridPreprocessingError> {
        use taffy::GridPlacement;

        let grid_size = if is_row {
            state.track_occupancy.grid_size.row
        } else {
            state.track_occupancy.grid_size.column
        };

        // Resolve start position
        let start = match line.start {
            GridPlacement::Auto => 0, // Will be resolved by auto-placement
            GridPlacement::Line(n) => {
                // Extract i16 value from GridLine and convert to i32
                let line_value = n.as_i16() as i32;
                // Negative lines count from end (1-indexed in CSS)
                if line_value < 0 {
                    (grid_size + line_value).max(0)
                } else {
                    // Convert 1-indexed to 0-indexed
                    (line_value - 1).max(0)
                }
            }
            GridPlacement::Span(_) => {
                // Span on start will be resolved after end
                0
            }
            GridPlacement::NamedLine(_, offset) => {
                // Named lines should be resolved before this point
                // For now, treat offset as absolute line position
                let line_value = offset as i32;
                if line_value < 0 {
                    (grid_size + line_value).max(0)
                } else {
                    (line_value - 1).max(0)
                }
            }
            GridPlacement::NamedSpan(_, _) => {
                // Named spans will be resolved after end
                0
            }
        };

        // Resolve end position
        let end = match line.end {
            GridPlacement::Auto => {
                // If start is explicit, end = start + 1
                match line.start {
                    GridPlacement::Span(n) => n as i32, // Span from 0
                    _ => start + 1,
                }
            }
            GridPlacement::Line(n) => {
                let line_value = n.as_i16() as i32;
                if line_value < 0 {
                    (grid_size + line_value).max(start)
                } else {
                    (line_value - 1).max(start)
                }
            }
            GridPlacement::Span(n) => {
                // Span n tracks from start
                start + n as i32
            }
            GridPlacement::NamedLine(_, offset) => {
                // Named lines should be resolved before this point
                let line_value = offset as i32;
                if line_value < 0 {
                    (grid_size + line_value).max(start)
                } else {
                    (line_value - 1).max(start)
                }
            }
            GridPlacement::NamedSpan(_, span_count) => {
                // Named span: span n tracks from start
                start + span_count as i32
            }
        };

        // Handle span on start (end is explicit)
        let final_start = match line.start {
            GridPlacement::Span(n) => (end - n as i32).max(0),
            _ => start,
        };

        Ok((final_start, end))
    }

    /// Helper: Map contributions to parent tracks
    pub fn map_contributions_to_parent_tracks(
        &self,
        _subgrid_id: NodeId,
        contributions: Vec<TrackSizeContribution>,
    ) -> Result<Vec<TrackSizeContribution>, GridPreprocessingError> {
        Ok(contributions)
    }

    /// Helper: Update parent track sizing
    pub fn update_parent_track_sizing(
        &mut self,
        _tree: &crate::BaseDocument,
        subgrid_id: NodeId,
        contributions: Vec<TrackSizeContribution>,
    ) -> Result<bool, GridPreprocessingError> {
        let mut any_changes = false;
        
        // Get parent grid ID from subgrid state
        let parent_grid_id = self.subgrid_states.get(&subgrid_id)
            .and_then(|state| state.layout_pass_state.dependencies.first())
            .copied()
            .ok_or_else(|| GridPreprocessingError::preprocessing_failed(
                "update_parent_tracks",
                subgrid_id.into(),
                "No parent grid found"
            ))?;
        
        // Access parent's intrinsic sizing state from subgrid module
        // Initialize if not present
        if !self.intrinsic_sizing_states.contains_key(&parent_grid_id) {
            self.intrinsic_sizing_states.insert(parent_grid_id, IntrinsicSizingState {
                content_contributions: std::collections::HashMap::new(),
                track_size_requirements: Vec::new(),
                sizing_pass_state: SizingPassState::default(),
            });
        }
        
        for contribution in contributions {
            // Get mutable reference to parent's sizing state
            let parent_state = self.intrinsic_sizing_states.get_mut(&parent_grid_id)
                .ok_or_else(|| GridPreprocessingError::preprocessing_failed(
                    "update_parent_tracks",
                    parent_grid_id.into(),
                    "Parent sizing state not found"
                ))?;
            
            // Ensure track size requirement exists for this contribution
            while parent_state.track_size_requirements.len() <= contribution.parent_track_index {
                parent_state.track_size_requirements.push(TrackSizeRequirement {
                    track_index: parent_state.track_size_requirements.len(),
                    min_size: 0.0,
                    max_size: 0.0,
                    flex_factor: 0.0,
                    axis: contribution.axis,
                });
            }
            
            // Update track size if contribution is larger
            let requirement = &mut parent_state.track_size_requirements[contribution.parent_track_index];
            let current_size = requirement.min_size;
            let new_size = current_size.max(contribution.min_size);
            
            if (new_size - current_size).abs() > 0.1 {
                requirement.min_size = new_size;
                requirement.max_size = requirement.max_size.max(contribution.max_size);
                any_changes = true;
            }
        }
        
        Ok(any_changes)
    }

    /// Helper: Trigger parent recompute
    pub fn trigger_parent_recompute(
        &self,
        _subgrid_id: NodeId,
    ) -> Result<(), GridPreprocessingError> {
        Ok(())
    }
}

/// Convert taffy TrackSizingFunction to internal TrackSizingFunction
/// 
/// Handles all variants of taffy::TrackSizingFunction (MinMax<MinTrackSizingFunction, MaxTrackSizingFunction>)
/// and converts them to internal Blitz grid representation following CSS Grid specification.
fn convert_taffy_sizing_function_to_internal(
    track_fn: &taffy::TrackSizingFunction,
    parent_size: taffy::Size<Option<f32>>,
    axis: GridAxis,
) -> TrackSizingFunction {
    let min_fn = track_fn.min_sizing_function();
    let max_fn = track_fn.max_sizing_function();
    
    // Select the correct dimension based on track axis
    // Row tracks use height, column tracks use width
    let parent_size_for_track = match axis {
        GridAxis::Row => parent_size.height,
        GridAxis::Column => parent_size.width,
    };
    
    // Handle different combinations of min/max track sizing functions
    match (
        extract_min_size_value(&min_fn, parent_size_for_track),
        extract_max_size_value(&max_fn, parent_size_for_track)
    ) {
        // Both min and max are fixed sizes - use MinMax
        (Some((min_size, _)), Some((max_size, _))) => {
            TrackSizingFunction {
                function_type: SizingFunctionType::MinMax(min_size, max_size),
                sizes: vec![min_size, max_size],
                flex_factor: None,
            }
        }
        
        // Max has Fr unit - this is a flexible track
        (_, Some((_, Some(fr_value)))) => {
            TrackSizingFunction {
                function_type: SizingFunctionType::Fr(fr_value),
                sizes: vec![],
                flex_factor: Some(fr_value),
            }
        }
        
        // Only min has fixed size, max is intrinsic
        (Some((min_size, _)), None) => {
            TrackSizingFunction {
                function_type: SizingFunctionType::Fixed(min_size),
                sizes: vec![min_size],
                flex_factor: None,
            }
        }
        
        // Only max has fixed size, min is intrinsic  
        (None, Some((max_size, _))) => {
            TrackSizingFunction {
                function_type: SizingFunctionType::Fixed(max_size),
                sizes: vec![max_size],
                flex_factor: None,
            }
        }
        
        // Both are intrinsic (auto, min-content, max-content, fit-content)
        (None, None) => {
            // Check for fit-content specifically
            if max_fn.is_fit_content() {
                if let Some(fit_content_limit) = extract_fit_content_limit(&max_fn, parent_size_for_track) {
                    TrackSizingFunction {
                        function_type: SizingFunctionType::FitContent(fit_content_limit),
                        sizes: vec![fit_content_limit],
                        flex_factor: None,
                    }
                } else {
                    // Fallback for fit-content without determinable limit
                    TrackSizingFunction {
                        function_type: SizingFunctionType::Fixed(0.0),
                        sizes: vec![0.0],
                        flex_factor: None,
                    }
                }
            } else {
                // Auto, min-content, max-content - use auto sizing
                TrackSizingFunction {
                    function_type: SizingFunctionType::Fixed(0.0),
                    sizes: vec![0.0],
                    flex_factor: None,
                }
            }
        }
    }
}

/// Extract concrete size value from MinTrackSizingFunction
/// Returns (size_value, flex_factor) if the function has a definite size, None if intrinsic
fn extract_min_size_value(
    track_fn: &taffy::MinTrackSizingFunction,
    parent_size: Option<f32>,
) -> Option<(f32, Option<f32>)> {
    // For min track sizing function, check if it has a definite value
    if let Some(size) = track_fn.definite_value(parent_size, |_, _| 0.0) {
        Some((size, None))
    } else {
        None
    }
}

/// Extract size value from MaxTrackSizingFunction including Fr units
fn extract_max_size_value(
    track_fn: &taffy::MaxTrackSizingFunction,
    parent_size: Option<f32>,
) -> Option<(f32, Option<f32>)> {
    // Check for Fr units first
    if track_fn.is_fr() {
        // Extract actual fr value from the MaxTrackSizingFunction
        let fr_value = track_fn.into_raw().value();
        return Some((0.0, Some(fr_value)));
    }
    
    // Check for definite size value
    if let Some(size) = track_fn.definite_value(parent_size, |_, _| 0.0) {
        Some((size, None))
    } else {
        None
    }
}

/// Extract fit-content limit value from MaxTrackSizingFunction
fn extract_fit_content_limit(
    track_fn: &taffy::MaxTrackSizingFunction,
    parent_size: Option<f32>,
) -> Option<f32> {
    // Extract fit-content limit
    track_fn.definite_limit(parent_size, |_, _| 0.0)
}

/// Convert TrackSizingFunction to TrackDefinition for internal processing
/// 
/// Extracts concrete sizing information from taffy::TrackSizingFunction and converts
/// to a TrackDefinition with computed size constraints and track type classification.
fn convert_sizing_function_to_definition(track_fn: &taffy::TrackSizingFunction) -> TrackDefinition {
    let min_fn = track_fn.min_sizing_function();
    let max_fn = track_fn.max_sizing_function();
    
    // Extract size information from min and max functions (no parent context available here)
    let min_size_info = extract_min_size_value(&min_fn, None);
    let max_size_info = extract_max_size_value(&max_fn, None);
    
    // Determine track type based on the sizing functions
    let track_type = determine_track_type(&min_fn, &max_fn);
    
    // Calculate size constraints based on available information
    let (size, min_size, max_size) = calculate_size_constraints(min_size_info, max_size_info, &track_type);
    
    TrackDefinition {
        size,
        min_size,
        max_size,
        track_type,
    }
}

/// Determine the track type based on min and max sizing functions
fn determine_track_type(min_fn: &taffy::MinTrackSizingFunction, max_fn: &taffy::MaxTrackSizingFunction) -> TrackType {
    // Check for flexible (fr) units first
    if max_fn.is_fr() {
        return TrackType::Flexible;
    }
    
    // Check for intrinsic sizing
    if min_fn.is_min_content() || max_fn.is_min_content() {
        return TrackType::MinContent;
    }
    
    if min_fn.is_max_content() || max_fn.is_max_content() {
        return TrackType::MaxContent;
    }
    
    // Check for auto sizing
    if min_fn.is_auto() || max_fn.is_auto() {
        return TrackType::Auto;
    }
    
    // Default to fixed if we have definite sizes
    TrackType::Fixed
}

/// Calculate size constraints from min/max size information
fn calculate_size_constraints(
    min_size_info: Option<(f32, Option<f32>)>,
    max_size_info: Option<(f32, Option<f32>)>,
    track_type: &TrackType,
) -> (f32, f32, f32) {
    match track_type {
        TrackType::Fixed => {
            // For fixed tracks, use concrete size values
            let min_size = min_size_info.map(|(size, _)| size).unwrap_or(0.0);
            let max_size = max_size_info.map(|(size, _)| size).unwrap_or(min_size);
            
            // Primary size is the larger of min and max, or max if only max is available
            let size = max_size.max(min_size);
            
            (size, min_size, max_size)
        }
        
        TrackType::Flexible => {
            // For flexible tracks, size is determined during layout
            // Use 0 as base size, constraints will be applied during flex resolution
            (0.0, 0.0, f32::INFINITY)
        }
        
        TrackType::MinContent => {
            // Min-content tracks size to minimum content size
            // Use small base size that will be computed during layout
            (0.0, 0.0, f32::INFINITY)
        }
        
        TrackType::MaxContent => {
            // Max-content tracks size to maximum content size  
            // Use base size that will expand during layout
            (0.0, 0.0, f32::INFINITY)
        }
        
        TrackType::Auto => {
            // Auto tracks adapt between min-content and max-content
            // Start with 0 and let layout algorithm determine final size
            (0.0, 0.0, f32::INFINITY)
        }
    }
}

/// Convert internal TrackSizingFunction back to taffy::TrackSizingFunction
///
/// This is the reverse conversion from convert_taffy_sizing_function_to_internal(),
/// allowing us to apply inherited track definitions back to taffy's layout system.
pub fn convert_internal_to_taffy_sizing_function(
    internal_fn: &TrackSizingFunction,
) -> Result<taffy::TrackSizingFunction, GridPreprocessingError> {
    use taffy::style_helpers::*;
    
    match &internal_fn.function_type {
        SizingFunctionType::Fixed(size) => {
            // Fixed size - create taffy function with identical min/max
            Ok(taffy::TrackSizingFunction::from_length(*size))
        }
        
        SizingFunctionType::MinMax(min_size, max_size) => {
            // MinMax sizing - create taffy function with separate min/max constraints
            Ok(taffy::TrackSizingFunction {
                min: taffy::MinTrackSizingFunction::from_length(*min_size),
                max: taffy::MaxTrackSizingFunction::from_length(*max_size),
            })
        }
        
        SizingFunctionType::Fr(fr_value) => {
            // Flexible unit - create taffy function with fr on max, auto on min
            Ok(taffy::TrackSizingFunction::from_fr(*fr_value))
        }
        
        SizingFunctionType::FitContent(limit) => {
            // Fit-content function - create taffy fit-content with limit
            Ok(taffy::TrackSizingFunction::fit_content(length_percentage(*limit)))
        }
        
        SizingFunctionType::Repeat(count, nested_functions) => {
            // Repeat function - for subgrid inheritance, we need to consider the repeat count
            // Since we can only return one track function, we use the count to inform our choice
            if nested_functions.is_empty() {
                // Empty repeat - return auto sizing
                Ok(taffy::TrackSizingFunction::from_length(0.0))
            } else if *count == 0 {
                // Zero repeats - return minimal sizing
                Ok(taffy::TrackSizingFunction::from_length(0.0))
            } else {
                // Use count to select an appropriate representative function
                // For multiple repeats, we take the middle function to best represent the pattern
                let function_index = if nested_functions.len() == 1 {
                    0
                } else {
                    // Use count to influence selection - helps with pattern recognition
                    (*count as usize / 2) % nested_functions.len()
                };
                
                let selected_fn = &nested_functions[function_index];
                let nested_internal = TrackSizingFunction {
                    function_type: selected_fn.clone(),
                    sizes: internal_fn.sizes.clone(),
                    flex_factor: internal_fn.flex_factor,
                };
                convert_internal_to_taffy_sizing_function(&nested_internal)
            }
        }
    }
}

/// Create a taffy::LengthPercentage from a pixel value
/// Helper function for the internal-to-taffy conversion
fn length_percentage(pixels: f32) -> taffy::LengthPercentage {
    taffy::LengthPercentage::length(pixels)
}
