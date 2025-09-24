//! Enable the dom to lay itself out using taffy
//!
//! In servo, style and layout happen together during traversal
//! However, in Blitz, we do a style pass then a layout pass.
//! This is slower, yes, but happens fast enough that it's not a huge issue.

use style::values::computed::CSSPixelLength;
use style::values::computed::length_percentage::CalcLengthPercentage;

// Core layout modules
pub(crate) mod collect_inline_text;
pub(crate) mod construct;
pub(crate) mod inline;
pub(crate) mod intrinsic_sizing;
pub(crate) mod replaced;
pub(crate) mod style_cache;
pub(crate) mod stylo_to_blitz;
pub(crate) mod table;

// Decomposed layout modules
pub mod grid_context;
pub mod grid_coordination;
pub(crate) mod grid_errors;
pub(crate) mod grid_preprocessing;
pub(crate) mod layout_traits;
pub(crate) mod masonry;
pub mod subgrid;
pub(crate) mod tree_iteration;

// Re-exports for public API (only export what's actually used)
// These are available for internal use within the layout module

// Export grid layout coordinator from decomposed modules
// Export grid context types directly
pub use grid_context::ParentGridContext;
pub use grid_coordination::{
    AutoPlacementState, DensePackingState, GridArea, GridLayoutCoordinator, GridPosition,
    InheritedTrackDefinitions, IntrinsicSizeContribution, IntrinsicSizingState, ItemPlacement,
    LayoutPassState, LineNameMap, MasonryLayoutState, PlacementMethod, SizingFunctionType,
    SizingPassState, SubgridLayoutResult, SubgridLayoutState, TrackDefinition, TrackOccupancyMap,
    TrackSizeContribution, TrackSizeRequirement, TrackSizingFunction, TrackType,
    VirtualMasonryItem,
};
// Export decomposed subgrid types
pub use subgrid::{
    AutoPlacementCursor, FlowDirection, GridItemType, ItemSpan, MasonryFlowDirection,
    MasonryPosition, NestedSubgridCoordination, SubgridItem, SubgridItemPlacement,
    SubgridTrackInheritance, coordinate_nested_subgrids,
};

/// Utility function to resolve CSS calc() values during layout
pub(crate) fn resolve_calc_value(calc_ptr: *const (), parent_size: f32) -> f32 {
    let calc = unsafe { &*(calc_ptr as *const CalcLengthPercentage) };
    let result = calc.resolve(CSSPixelLength::new(parent_size));
    result.px()
}
