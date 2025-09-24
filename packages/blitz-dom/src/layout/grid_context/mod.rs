//! Grid context resolution and parent finding for CSS Grid Level 2 subgrid support
//!
//! This module provides the infrastructure for resolving parent grid contexts
//! and extracting grid track information from grid containers.

pub mod cache;
pub mod line_name_inheritance;
pub mod resolution;
pub mod track_extraction;
pub mod types;

// Re-export commonly used types and functions for convenient access
pub use cache::GridContextCache;
pub use line_name_inheritance::LineNameInheritanceMapper;
pub use resolution::{
    check_parent_grid_container, find_potential_parents_constrained,
    resolve_parent_grid_context_for_generic_tree,
    resolve_parent_grid_context_for_generic_tree_efficient,
};
pub use track_extraction::{
    detect_subgrid_axis_from_style, detect_subgrid_from_stylo, expand_repetition_pattern,
    extract_line_names_from_style, extract_line_names_from_stylo_computed_styles,
    extract_tracks_from_stylo_computed_styles, extract_tracks_from_template_list,
};
pub use types::{
    GridAxis, GridContextError, GridSpan, ParentGridContext, SubgridInheritanceLevel,
    TrackExtractionError,
};
