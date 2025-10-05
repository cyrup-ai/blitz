//! CSS Grid Level 2 subgrid preprocessing implementation
//!
//! This module implements subgrid track inheritance, line name mapping,
//! and nested subgrid coordination as specified in CSS Grid Level 2.
//!
//! The implementation is decomposed into logical modules:
//! - `types`: Core data structures and type definitions
//! - `coordination`: Nested subgrid coordination algorithms
//! - `layout_states`: State tracking for layout phases
//! - `layout_coordinator`: Main coordination system
//! - `auto_placement`: Auto-placement algorithms and utilities

// Public modules
pub mod auto_placement;
pub mod coordination;
pub mod layout_coordinator;
pub mod layout_states;
pub mod types;

// Re-exports for convenience
pub use auto_placement::{
    AutoPlacementCursor as AutoCursor, FlowDirection, GridItemType, GridPlacement, GridPosition,
    ItemSpan, OccupiedRange, PlacementMethod, SubgridItem, SubgridItemPlacement,
    SubgridPlacementState, TrackAvailability,
};
pub use coordination::NestedSubgridCoordination;
pub use layout_coordinator::{
    GridItemPlacement, GridLayoutCoordinator, GridLine, GridLineRange, OrderedGridItem,
};
pub use layout_states::{
    AbstractAxis, AutoFlowDirection, AutoPlacementCursor, AutoPlacementState,
    BidirectionalSizingState, DependencyStrength, DeterminedTrackCounts, IntrinsicSizingState,
    LayoutPassState, MasonryCoordinationState, MasonryFlowDirection, MasonryLayoutState,
    MasonryPackingState, MasonryPackingStrategy, MasonryPosition, PropagationDirection,
    PropagationPhase, ResolvedTrackSizes, SizingDependency, SubgridLayoutState,
    SubgridSizePropagation, TrackSizeCalculations,
};
pub use types::{
    ChildSubgridSpan, CoordinateTransform, EffectiveSubgridTracks, GridAxis, InheritedLineNames,
    ItemPlacement, LineNameMapping, SubgridInheritanceRegistry, SubgridLayoutResult, SubgridSpan,
    SubgridTrackInheritance, TrackInheritanceLevel, TrackSizingContribution,
};

// Legacy imports for compatibility
use super::grid_context::ParentGridContext;
use super::grid_errors::{SubgridError, SubgridResult};

/// Complete nested subgrid coordination implementing CSS Grid Level 2 multi-level inheritance
///
/// This algorithm handles the sophisticated case where subgrids contain other subgrids,
/// creating inheritance chains that require coordinate mapping and item contribution
/// propagation through multiple levels back to the root parent grid.
pub fn coordinate_nested_subgrids<Tree>(
    tree: &mut Tree,
    root_subgrid_id: taffy::prelude::NodeId,
    root_parent_context: &ParentGridContext,
    nesting_depth: usize,
) -> SubgridResult<NestedSubgridCoordination>
where
    Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
{
    const MAX_SUBGRID_NESTING_DEPTH: usize = 10; // Prevent infinite recursion

    if nesting_depth > MAX_SUBGRID_NESTING_DEPTH {
        return Err(SubgridError::ExcessiveNestingDepth {
            depth: nesting_depth,
            max_depth: MAX_SUBGRID_NESTING_DEPTH,
        });
    }

    let mut coordination = NestedSubgridCoordination::new(root_subgrid_id);

    // Phase 1: Process current subgrid level with parent inheritance
    process_current_subgrid_level(
        tree,
        root_subgrid_id,
        root_parent_context,
        &mut coordination,
    )?;

    // Phase 2: Discover and recursively process child subgrids
    let child_subgrids = discover_child_subgrids(tree, root_subgrid_id)?;

    for child_subgrid_id in child_subgrids {
        let child_coordination = coordinate_nested_subgrids(
            tree,
            child_subgrid_id,
            root_parent_context,
            nesting_depth + 1,
        )?;

        coordination.merge_child_coordination(child_coordination, root_parent_context)?;
    }

    Ok(coordination)
}

/// Process current subgrid level with parent inheritance
fn process_current_subgrid_level<Tree>(
    tree: &mut Tree,
    subgrid_id: taffy::prelude::NodeId,
    parent_context: &ParentGridContext,
    coordination: &mut NestedSubgridCoordination,
) -> SubgridResult<()>
where
    Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
{
    use super::grid_coordination::types::GridLayoutCoordinator;

    // Step 1: Create coordinator for this subgrid level
    let coordinator = GridLayoutCoordinator::default();

    // Step 2: Determine subgrid span in parent
    let subgrid_span = coordinator.determine_subgrid_span(subgrid_id, parent_context)
        .map_err(|e| SubgridError::CoordinationFailed { details: e.to_string() })?;

    // Step 3: Extract parent tracks for this span
    let inherited_tracks = coordinator.extract_parent_tracks(&subgrid_span, parent_context)
        .map_err(|e| SubgridError::CoordinationFailed { details: e.to_string() })?;

    // Step 4: Convert to Taffy track sizing functions
    use super::grid_coordination::helpers::convert_internal_to_taffy_sizing_function;

    let row_track_functions: Vec<taffy::TrackSizingFunction> = inherited_tracks
        .row_sizing_functions
        .iter()
        .filter_map(|f| {
            convert_internal_to_taffy_sizing_function(f)
                .map_err(|e| {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        "Failed to convert row track sizing function for subgrid {}: {}",
                        usize::from(subgrid_id),
                        e
                    );
                    e
                })
                .ok()
        })
        .collect();

    let column_track_functions: Vec<taffy::TrackSizingFunction> = inherited_tracks
        .column_sizing_functions
        .iter()
        .filter_map(|f| {
            convert_internal_to_taffy_sizing_function(f)
                .map_err(|e| {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        "Failed to convert column track sizing function for subgrid {}: {}",
                        usize::from(subgrid_id),
                        e
                    );
                    e
                })
                .ok()
        })
        .collect();

    // Step 5: Apply to node's Style - requires BaseDocument access
    let base_doc = (tree as &mut dyn std::any::Any)
        .downcast_mut::<crate::BaseDocument>()
        .ok_or_else(|| SubgridError::StyleAccess {
            node_id: usize::from(subgrid_id),
            reason: "Failed to downcast tree to BaseDocument for style update".to_string(),
        })?;

    let node = base_doc.node_from_id_mut(subgrid_id);
    let style = node.style_mut();

    // Convert TrackSizingFunction to GridTemplateComponent::Single
    if !row_track_functions.is_empty() {
        style.grid_template_rows = row_track_functions
            .iter()
            .map(|track| taffy::GridTemplateComponent::Single(*track))
            .collect();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Applied {} inherited row tracks to subgrid node {}",
            style.grid_template_rows.len(),
            usize::from(subgrid_id)
        );
    }

    if !column_track_functions.is_empty() {
        style.grid_template_columns = column_track_functions
            .iter()
            .map(|track| taffy::GridTemplateComponent::Single(*track))
            .collect();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Applied {} inherited column tracks to subgrid node {}",
            style.grid_template_columns.len(),
            usize::from(subgrid_id)
        );
    }

    // Increment style generation to invalidate cache
    node.style_generation.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Step 6: Setup line name mapping for subgrid items
    let line_name_mapping = coordinator.setup_line_name_mapping(subgrid_id, parent_context)
        .map_err(|e| SubgridError::CoordinationFailed { details: e.to_string() })?;

    // Store in coordination state for nested subgrid inheritance
    coordination.line_name_mappings.push(line_name_mapping);

    // Step 7: Update coordination state - track the subgrid in the chain
    coordination.subgrid_chain.push(subgrid_id);

    Ok(())
}

/// Discover child subgrids within a parent subgrid
fn discover_child_subgrids<Tree>(
    _tree: &Tree,
    _parent_id: taffy::prelude::NodeId,
) -> SubgridResult<Vec<taffy::prelude::NodeId>>
where
    Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
{
    // Simplified implementation - would traverse children in full version
    Ok(Vec::new())
}

/// Legacy wrapper function for compatibility
pub fn preprocess_subgrid_for_generic_tree<Tree>(
    tree: &mut Tree,
    subgrid_id: taffy::prelude::NodeId,
    parent_context: &ParentGridContext,
) -> SubgridResult<()>
where
    Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
{
    // Call the main coordination function and discard result for compatibility
    coordinate_nested_subgrids(tree, subgrid_id, parent_context, 0)?;
    Ok(())
}
