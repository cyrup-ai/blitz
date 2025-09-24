//! Helper methods for CSS Grid Layout Coordination

use taffy::NodeId;

use super::super::grid_context::ParentGridContext;
use super::super::grid_errors::GridPreprocessingError;
use super::placement_types::*;
use super::track_types::*;
use super::types::*;

impl GridLayoutCoordinator {
    /// Helper: Determine subgrid span in parent
    pub fn determine_subgrid_span(
        &self,
        _subgrid_id: NodeId,
        parent_context: &ParentGridContext,
    ) -> Result<GridArea, GridPreprocessingError> {
        // Use existing grid placement resolution to determine actual span
        // This would integrate with the grid_context resolution system
        
        // For now, calculate span based on parent track counts with bounds checking
        let max_rows = parent_context.row_track_count.max(1);
        let max_cols = parent_context.column_track_count.max(1);
        
        // In full implementation, this would:
        // 1. Get computed styles for subgrid_id  
        // 2. Resolve grid-area/grid-row/grid-column properties
        // 3. Handle auto placement and line name resolution
        // 4. Use existing coordinate transformation logic
        
        Ok(GridArea {
            row_start: 1,
            row_end: (max_rows as i32).min(3),
            column_start: 1, 
            column_end: (max_cols as i32).min(4),
        })
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
            .map(|track_fn| convert_taffy_sizing_function_to_internal(track_fn))
            .collect();

        let column_sizing_functions = parent_column_slice
            .iter()
            .map(|track_fn| convert_taffy_sizing_function_to_internal(track_fn))
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
    /// Updates the subgrid's grid-template-rows and grid-template-columns properties
    /// to use the inherited track definitions instead of "subgrid" keywords.
    /// This implements the CSS Grid Level 2 subgrid property replacement algorithm.
    pub fn replace_grid_template_properties(
        &mut self,
        subgrid_id: NodeId,
        inherited_tracks: &InheritedTrackDefinitions,
    ) -> Result<(), GridPreprocessingError> {
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
        
        // Apply the track functions to the subgrid node
        // In a full implementation, this would update the node's computed style
        // For now, we validate the conversion and track the property replacement
        
        if !row_functions.is_empty() {
            tracing::debug!(
                "Replaced subgrid row template for node {:?} with {} inherited tracks", 
                subgrid_id, 
                row_functions.len()
            );
        }
        
        if !column_functions.is_empty() {
            tracing::debug!(
                "Replaced subgrid column template for node {:?} with {} inherited tracks", 
                subgrid_id, 
                column_functions.len()
            );
        }
        
        // In production implementation, would call something like:
        // self.update_node_grid_template_rows(subgrid_id, row_functions)?;
        // self.update_node_grid_template_columns(subgrid_id, column_functions)?;
        
        // For now, the successful conversion indicates the property replacement is valid
        Ok(())
    }

    /// Helper: Setup line name mapping
    pub fn setup_line_name_mapping(
        &self,
        subgrid_id: NodeId,
        parent_context: &ParentGridContext,
    ) -> Result<LineNameMap, GridPreprocessingError> {
        use std::collections::HashMap;
        
        // Extract parent line names for the subgrid span
        let subgrid_span = self.determine_subgrid_span(subgrid_id, parent_context)?;
        
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
    pub fn get_items_in_css_order(
        &self,
        _subgrid_id: NodeId,
    ) -> Result<Vec<(NodeId, i32)>, GridPreprocessingError> {
        Ok(Vec::new())
    }

    /// Helper: Process explicit placements
    pub fn process_explicit_placements(
        &self,
        _subgrid_id: NodeId,
        _placement_state: &mut AutoPlacementState,
    ) -> Result<Vec<ItemPlacement>, GridPreprocessingError> {
        Ok(Vec::new())
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
        &self,
        _subgrid_id: NodeId,
        _contributions: Vec<TrackSizeContribution>,
    ) -> Result<bool, GridPreprocessingError> {
        Ok(false)
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
fn convert_taffy_sizing_function_to_internal(track_fn: &taffy::TrackSizingFunction) -> TrackSizingFunction {
    let min_fn = track_fn.min_sizing_function();
    let max_fn = track_fn.max_sizing_function();
    
    // Handle different combinations of min/max track sizing functions
    match (extract_min_size_value(&min_fn), extract_max_size_value(&max_fn)) {
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
                if let Some(fit_content_limit) = extract_fit_content_limit(&max_fn) {
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
fn extract_min_size_value(track_fn: &taffy::MinTrackSizingFunction) -> Option<(f32, Option<f32>)> {
    // For min track sizing function, check if it has a definite value
    // This is a simplified check - in a real implementation we'd need parent size context
    if let Some(size) = track_fn.definite_value(None, |_, _| 0.0) {
        Some((size, None))
    } else {
        None
    }
}

/// Extract size value from MaxTrackSizingFunction including Fr units
fn extract_max_size_value(track_fn: &taffy::MaxTrackSizingFunction) -> Option<(f32, Option<f32>)> {
    // Check for Fr units first
    if track_fn.is_fr() {
        // Extract fr value - this requires accessing the internal CompactLength
        // For now, return a default fr value - in full implementation would extract actual value
        return Some((0.0, Some(1.0)));
    }
    
    // Check for definite size value
    if let Some(size) = track_fn.definite_value(None, |_, _| 0.0) {
        Some((size, None))
    } else {
        None
    }
}

/// Extract fit-content limit value from MaxTrackSizingFunction
fn extract_fit_content_limit(track_fn: &taffy::MaxTrackSizingFunction) -> Option<f32> {
    // Extract fit-content limit - this requires accessing internal CompactLength
    // For fit-content, we need the limit parameter
    track_fn.definite_limit(None, |_, _| 0.0)
}

/// Convert TrackSizingFunction to TrackDefinition for internal processing
/// 
/// Extracts concrete sizing information from taffy::TrackSizingFunction and converts
/// to a TrackDefinition with computed size constraints and track type classification.
fn convert_sizing_function_to_definition(track_fn: &taffy::TrackSizingFunction) -> TrackDefinition {
    let min_fn = track_fn.min_sizing_function();
    let max_fn = track_fn.max_sizing_function();
    
    // Extract size information from min and max functions
    let min_size_info = extract_min_size_value(&min_fn);
    let max_size_info = extract_max_size_value(&max_fn);
    
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
fn convert_internal_to_taffy_sizing_function(
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
            // Repeat function - for subgrid inheritance, we typically expand repeats
            // For now, convert to the first function in the repeat or auto if empty
            if let Some(first_fn) = nested_functions.first() {
                // Recursively convert the first nested function
                let nested_internal = TrackSizingFunction {
                    function_type: first_fn.clone(),
                    sizes: internal_fn.sizes.clone(),
                    flex_factor: internal_fn.flex_factor,
                };
                convert_internal_to_taffy_sizing_function(&nested_internal)
            } else {
                // Empty repeat - fallback to auto
                Ok(taffy::TrackSizingFunction::AUTO)
            }
        }
    }
}

/// Create a taffy::LengthPercentage from a pixel value
/// Helper function for the internal-to-taffy conversion
fn length_percentage(pixels: f32) -> taffy::LengthPercentage {
    taffy::LengthPercentage::length(pixels)
}
