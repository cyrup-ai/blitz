//! Nested subgrid coordination state and algorithms
//!
//! This module implements the coordination between nested subgrids, handling
//! inheritance chains and cross-axis dependencies as specified in CSS Grid Level 2.

use taffy::prelude::*;

use super::super::grid_context::ParentGridContext;
use super::super::grid_errors::{SubgridError, SubgridResult};
use super::types::*;

/// Nested subgrid coordination state and results
///
/// Tracks the complete inheritance chain and accumulated item contributions
/// from all levels of nested subgrids back to the root parent grid.
#[derive(Debug, Clone)]
pub struct NestedSubgridCoordination {
    /// Root subgrid node ID for this coordination
    pub root_subgrid_id: NodeId,
    /// Complete subgrid inheritance chain (root to deepest)
    pub subgrid_chain: Vec<NodeId>,
    /// Track inheritance at each level of the chain
    pub track_inheritance_chain: Vec<TrackInheritanceLevel>,
    /// Current effective tracks after all inheritance applied
    pub current_effective_tracks: EffectiveSubgridTracks,
    /// All item contributions mapped to root parent coordinates
    pub item_contributions: Vec<TrackSizingContribution>,
    /// Line name mappings through the inheritance chain
    pub line_name_mappings: Vec<LineNameMapping>,
}

impl NestedSubgridCoordination {
    /// Create new coordination state for a root subgrid
    pub fn new(root_subgrid_id: NodeId) -> Self {
        Self {
            root_subgrid_id,
            subgrid_chain: vec![root_subgrid_id],
            track_inheritance_chain: Vec::new(),
            current_effective_tracks: EffectiveSubgridTracks {
                effective_row_tracks: Vec::new(),
                effective_column_tracks: Vec::new(),
                effective_row_line_names: Vec::new(),
                effective_column_line_names: Vec::new(),
            },
            item_contributions: Vec::new(),
            line_name_mappings: Vec::new(),
        }
    }

    /// Merge child coordination results into parent coordination
    ///
    /// This handles the complex case where child subgrid items need to contribute
    /// their sizing requirements back to the root parent grid through multiple
    /// levels of coordinate transformation.
    pub fn merge_child_coordination(
        &mut self,
        child_coordination: NestedSubgridCoordination,
        root_parent_context: &ParentGridContext,
    ) -> SubgridResult<()> {
        // Phase 1: Extend the subgrid inheritance chain
        self.subgrid_chain.extend(child_coordination.subgrid_chain);

        // Phase 2: Merge track inheritance levels (maintains chain order)
        self.track_inheritance_chain
            .extend(child_coordination.track_inheritance_chain);

        // Phase 3: Map ALL child item contributions to root parent coordinates
        // This is the critical algorithm for nested subgrid coordination
        for child_contribution in child_coordination.item_contributions {
            let mapped_contribution = map_contribution_through_inheritance_chain(
                child_contribution,
                &self.track_inheritance_chain,
                root_parent_context,
            )?;
            self.item_contributions.push(mapped_contribution);
        }

        // Phase 4: Merge line name mappings (preserves inheritance order)
        self.line_name_mappings
            .extend(child_coordination.line_name_mappings);

        Ok(())
    }
}

/// Map item contribution through inheritance chain to root parent coordinates
fn map_contribution_through_inheritance_chain(
    contribution: TrackSizingContribution,
    inheritance_chain: &[TrackInheritanceLevel],
    root_parent_context: &ParentGridContext,
) -> SubgridResult<TrackSizingContribution> {
    let mut mapped_contribution = contribution;

    // Transform through each level in the inheritance chain
    for level in inheritance_chain {
        mapped_contribution =
            apply_coordinate_transform(mapped_contribution, &level.coordinate_transform)?;
    }

    // Validate against root parent grid bounds
    validate_contribution_bounds(&mapped_contribution, root_parent_context)?;

    Ok(mapped_contribution)
}

/// Apply coordinate transformation to a track sizing contribution
fn apply_coordinate_transform(
    mut contribution: TrackSizingContribution,
    transform: &CoordinateTransform,
) -> SubgridResult<TrackSizingContribution> {
    match contribution.axis {
        GridAxis::Row => {
            contribution.track_index = contribution
                .track_index
                .checked_add(transform.row_offset)
                .ok_or_else(|| SubgridError::CoordinateMappingFailed {
                details: "Row offset overflow in coordinate transformation".to_string(),
            })?;

            // Apply row scaling if needed
            contribution.min_size *= transform.row_scale;
            contribution.max_size *= transform.row_scale;
            if let Some(ref mut preferred) = contribution.preferred_size {
                *preferred *= transform.row_scale;
            }
        }
        GridAxis::Column => {
            contribution.track_index = contribution
                .track_index
                .checked_add(transform.column_offset)
                .ok_or_else(|| SubgridError::CoordinateMappingFailed {
                    details: "Column offset overflow in coordinate transformation".to_string(),
                })?;

            // Apply column scaling if needed
            contribution.min_size *= transform.column_scale;
            contribution.max_size *= transform.column_scale;
            if let Some(ref mut preferred) = contribution.preferred_size {
                *preferred *= transform.column_scale;
            }
        }
    }

    Ok(contribution)
}

/// Validate contribution bounds against root parent context
fn validate_contribution_bounds(
    contribution: &TrackSizingContribution,
    root_parent_context: &ParentGridContext,
) -> SubgridResult<()> {
    match contribution.axis {
        GridAxis::Row => {
            if contribution.track_index >= root_parent_context.row_track_count {
                return Err(SubgridError::CoordinateMappingFailed {
                    details: format!(
                        "Row track index {} exceeds parent grid track count {}",
                        contribution.track_index, root_parent_context.row_track_count
                    ),
                });
            }
        }
        GridAxis::Column => {
            if contribution.track_index >= root_parent_context.column_track_count {
                return Err(SubgridError::CoordinateMappingFailed {
                    details: format!(
                        "Column track index {} exceeds parent grid track count {}",
                        contribution.track_index, root_parent_context.column_track_count
                    ),
                });
            }
        }
    }

    Ok(())
}
