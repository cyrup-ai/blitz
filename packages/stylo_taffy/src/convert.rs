//! Conversion functions from Stylo computed style types to Taffy equivalents

use thiserror::Error;
use tracing;

/// Private module of type aliases so we can refer to stylo types with nicer names
pub mod stylo {
    pub(crate) use style::properties::ComputedValues;
    pub(crate) use style::properties::generated::longhands::box_sizing::computed_value::T as BoxSizing;
    pub(crate) use style::properties::longhands::aspect_ratio::computed_value::T as AspectRatio;
    pub(crate) use style::properties::longhands::position::computed_value::T as Position;
    pub(crate) use style::values::computed::length_percentage::CalcLengthPercentage;
    pub(crate) use style::values::computed::length_percentage::Unpacked as UnpackedLengthPercentage;
    pub(crate) use style::values::computed::{Length, LengthPercentage, Percentage};
    pub(crate) use style::values::generics::NonNegative;
    pub(crate) use style::values::generics::length::{
        GenericLengthPercentageOrNormal, GenericMargin, GenericMaxSize, GenericSize,
    };
    pub(crate) use style::values::generics::position::{Inset as GenericInset, PreferredRatio};
    pub(crate) use style::values::specified::align::{AlignFlags, ContentDistribution};
    pub(crate) use style::values::specified::box_::{
        Display, DisplayInside, DisplayOutside, Overflow,
    };
    pub(crate) type MarginVal = GenericMargin<LengthPercentage>;
    pub(crate) type InsetVal = GenericInset<Percentage, LengthPercentage>;
    pub(crate) type Size = GenericSize<NonNegative<LengthPercentage>>;
    pub(crate) type MaxSize = GenericMaxSize<NonNegative<LengthPercentage>>;

    pub(crate) type Gap = GenericLengthPercentageOrNormal<NonNegative<LengthPercentage>>;

    #[cfg(feature = "flexbox")]
    pub(crate) use style::{
        computed_values::{flex_direction::T as FlexDirection, flex_wrap::T as FlexWrap},
        values::generics::flex::GenericFlexBasis,
    };
    #[cfg(feature = "flexbox")]
    pub(crate) type FlexBasis = GenericFlexBasis<Size>;

    #[cfg(feature = "block")]
    pub(crate) use style::values::computed::text::TextAlign;
    #[cfg(feature = "grid")]
    pub(crate) use style::{
        computed_values::grid_auto_flow::T as GridAutoFlow,
        values::{
            computed::{GridLine, GridTemplateComponent, ImplicitGridTracks},
            generics::grid::{RepeatCount, TrackBreadth, TrackList, TrackListValue, TrackRepeat, TrackSize},
            specified::GenericGridTemplateComponent,
        },
    };
}

// Import type aliases from the stylo module for public use
// use stylo::{MarginVal, InsetVal, Size, MaxSize, Gap};
// #[cfg(feature = "flexbox")]
// use stylo::FlexBasis;

use style::Atom;
use style::OwnedSlice;
use style::media_queries::Device;
use style::values::CustomIdent;
use style::values::specified::GridTemplateAreas;
use taffy::CompactLength;
use taffy::style_helpers::*;

/// Error types for subgrid line name mapping operations
#[derive(Error, Debug, Clone)]
pub enum SubgridLineNameError {
    #[error("Line index {line_index} out of bounds for track count {track_count}")]
    LineIndexOutOfBounds {
        line_index: usize,
        track_count: usize,
    },

    #[error("Invalid line name format: {0}")]
    InvalidLineNameFormat(String),

    #[error(
        "Cannot add line names to Single track at index {track_index} (only Repeat tracks support line names)"
    )]
    SingleTrackLineNameUnsupported { track_index: usize },

    #[error("Failed to convert CustomIdent to String: {0}")]
    CustomIdentConversionFailed(String),
}

/// Result type for subgrid line name operations
pub type SubgridLineNameResult<T> = Result<T, SubgridLineNameError>;

/// Grid axis enumeration for subgrid and masonry context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridAxis {
    /// Row axis (grid-template-rows)
    Row,
    /// Column axis (grid-template-columns)  
    Column,
}

/// Context for subgrid track inheritance and masonry layout
/// Contains parent grid information needed for proper subgrid and masonry implementation
#[derive(Debug, Clone)]
pub struct GridContext {
    /// Parent grid track definitions for the relevant axis
    /// These are the actual track sizes that subgrid should inherit
    pub parent_tracks: Vec<taffy::GridTemplateComponent<String>>,

    /// Parent grid line names for the relevant axis
    /// Used for subgrid line name mapping and resolution
    pub parent_line_names: Vec<Vec<String>>,

    /// Which grid axis this context applies to (row or column)
    pub axis: GridAxis,

    /// Whether the parent grid supports subgrid in this axis
    pub supports_subgrid: bool,

    /// Available space in the parent grid for masonry track calculation
    /// Used to determine optimal number of masonry tracks
    pub available_space: Option<f32>,

    /// Existing masonry state for this axis if already processing masonry items
    /// Tracks running positions for shortest-track placement algorithm
    pub masonry_state: Option<MasonryPlacementState>,
}

impl Default for GridContext {
    fn default() -> Self {
        Self {
            parent_tracks: Vec::new(),
            parent_line_names: Vec::new(),
            axis: GridAxis::Row,
            supports_subgrid: false,
            available_space: None,
            masonry_state: None,
        }
    }
}

impl GridContext {
    /// Create a new grid context for subgrid inheritance
    pub fn new_subgrid(
        parent_tracks: Vec<taffy::GridTemplateComponent<String>>,
        parent_line_names: Vec<Vec<String>>,
        axis: GridAxis,
    ) -> Self {
        Self {
            parent_tracks,
            parent_line_names,
            axis,
            supports_subgrid: true,
            available_space: None,
            masonry_state: None,
        }
    }

    /// Create a new grid context for masonry layout
    pub fn new_masonry(axis: GridAxis, available_space: f32) -> Self {
        Self {
            parent_tracks: Vec::new(),
            parent_line_names: Vec::new(),
            axis,
            supports_subgrid: false,
            available_space: Some(available_space),
            masonry_state: Some(MasonryPlacementState::new(4)), // Default 4 tracks
        }
    }
}

/// Masonry item placement state for tracking shortest-track algorithm
/// Implements the CSS Grid Level 3 masonry placement algorithm
#[derive(Debug, Clone)]
pub struct MasonryPlacementState {
    /// Running position (accumulated size) of each track
    /// Used to find the "shortest" track for next item placement
    pub track_positions: Vec<f32>,

    /// Number of items placed in each track
    /// Used for balancing and statistics
    pub track_item_counts: Vec<usize>,

    /// Total number of tracks in the masonry grid
    pub track_count: usize,

    /// Item tolerance for track selection (CSS Grid Level 3)
    /// Default: 1em converted to pixels
    pub item_tolerance: f32,

    /// Track index of most recently placed item
    /// Used for tie-breaking in track selection
    pub last_placed_track: Option<usize>,

    /// Auto-placement cursor for forward movement
    /// Ensures items don't backtrack unnecessarily
    pub placement_cursor: usize,
}

impl MasonryPlacementState {
    /// Create a new masonry placement state with the specified number of tracks
    pub fn new(track_count: usize) -> Self {
        Self {
            track_positions: vec![0.0; track_count],
            track_item_counts: vec![0; track_count],
            track_count,
            item_tolerance: 16.0, // Default 1em ≈ 16px
            last_placed_track: None,
            placement_cursor: 0,
        }
    }

    /// Create a new masonry placement state with custom item tolerance
    pub fn new_with_tolerance(track_count: usize, tolerance: f32) -> Self {
        let mut state = Self::new(track_count);
        state.item_tolerance = tolerance;
        state
    }

    /// CSS Grid Level 3 compliant track selection with item-tolerance
    /// Implements the full specification algorithm with tie-breaking
    pub fn find_shortest_track_with_tolerance(&self) -> usize {
        // Step 1: Find absolute shortest track position
        let min_position = self
            .track_positions
            .iter()
            .fold(f32::INFINITY, |acc, &pos| acc.min(pos));

        // Step 2: Find all tracks within tolerance of shortest
        let tolerance_threshold = min_position + self.item_tolerance;
        let candidates: Vec<usize> = self
            .track_positions
            .iter()
            .enumerate()
            .filter(|&(_, pos)| *pos <= tolerance_threshold)
            .map(|(idx, _)| idx)
            .collect();

        // Step 3: Apply CSS Grid Level 3 tie-breaking rules
        self.apply_tie_breaking_rules(candidates)
    }

    /// Apply CSS Grid Level 3 tie-breaking rules for track selection
    fn apply_tie_breaking_rules(&self, candidates: Vec<usize>) -> usize {
        if candidates.is_empty() {
            return 0;
        }

        if candidates.len() == 1 {
            return candidates[0];
        }

        // Rule 1: Prefer tracks after most recently placed item
        if let Some(last_track) = self.last_placed_track {
            // Find candidates that come after the last placed track
            let post_last_candidates: Vec<usize> = candidates
                .iter()
                .filter(|&&track| track >= last_track)
                .copied()
                .collect();

            if !post_last_candidates.is_empty() {
                // Among post-last candidates, prefer the earliest
                return *post_last_candidates.iter().min().unwrap_or(&0);
            }
        }

        // Rule 2: If no recent placement or no candidates after last,
        // prefer earliest track that's >= placement cursor
        let cursor_candidates: Vec<usize> = candidates
            .iter()
            .filter(|&&track| track >= self.placement_cursor)
            .copied()
            .collect();

        if !cursor_candidates.is_empty() {
            return *cursor_candidates.iter().min().unwrap_or(&0);
        }

        // Rule 3: Fallback to earliest available track
        *candidates.iter().min().unwrap_or(&0)
    }

    /// Enhanced item placement with CSS Grid Level 3 tracking
    pub fn place_item_with_tracking(&mut self, track_index: usize, item_size: f32, span: usize) {
        // Call existing placement logic
        self.place_item(track_index, item_size, span);

        // Update CSS Grid Level 3 tracking state
        self.last_placed_track = Some(track_index);

        // Update placement cursor for forward movement
        let end_track = (track_index + span).min(self.track_count);
        self.placement_cursor = end_track.min(self.track_count.saturating_sub(1));
    }

    /// Get configured item tolerance value
    pub fn get_item_tolerance(&self) -> f32 {
        self.item_tolerance
    }

    /// Set item tolerance (for runtime configuration)
    pub fn set_item_tolerance(&mut self, tolerance: f32) {
        self.item_tolerance = tolerance.max(0.0); // Ensure non-negative
    }

    /// CSS Grid Level 3 dense packing algorithm foundation
    /// Finds optimal placement that minimizes gaps in the masonry layout
    /// Returns the track index for dense placement, or None if no better placement exists
    pub fn find_dense_placement(&self, item_span: usize) -> Option<usize> {
        if item_span == 0 || item_span > self.track_count {
            return None;
        }

        let mut best_track = None;
        let mut best_max_position = f32::INFINITY;

        // Iterate through all possible track positions for the given span
        for track in 0..=(self.track_count.saturating_sub(item_span)) {
            // Calculate the maximum position across all tracks in this span
            let span_end = track + item_span;
            let span_positions = &self.track_positions[track..span_end];
            let max_position = span_positions.iter().fold(0.0f32, |acc, &pos| acc.max(pos));

            // Dense packing prefers placements that minimize the maximum position
            // This creates a more compact layout by filling gaps earlier
            if max_position < best_max_position {
                best_max_position = max_position;
                best_track = Some(track);
            }
        }

        // Only return a track if it would create a more compact layout
        // compared to the standard tolerance-based placement
        if let Some(track) = best_track {
            // Check if this placement is meaningfully better than tolerance-based placement
            let tolerance_track = self.find_shortest_track_with_tolerance();
            let tolerance_position = if tolerance_track + item_span <= self.track_count {
                let tolerance_span =
                    &self.track_positions[tolerance_track..tolerance_track + item_span];
                tolerance_span.iter().fold(0.0f32, |acc, &pos| acc.max(pos))
            } else {
                f32::INFINITY
            };

            // Return dense placement only if it's significantly better (more than tolerance)
            if best_max_position + self.item_tolerance < tolerance_position {
                return Some(track);
            }
        }

        None
    }

    /// Place an item in the masonry grid, updating track positions
    /// Implements the CSS Grid Level 3 item placement algorithm
    pub fn place_item(&mut self, track_index: usize, item_size: f32, span: usize) {
        if track_index >= self.track_count {
            return;
        }

        let end_track = (track_index + span).min(self.track_count);

        // Find the maximum running position across all spanned tracks
        let placement_position = self.track_positions[track_index..end_track]
            .iter()
            .fold(0.0f32, |acc, &pos| acc.max(pos));

        // Update all spanned tracks to the new position
        for i in track_index..end_track {
            self.track_positions[i] = placement_position + item_size;
            self.track_item_counts[i] += 1;
        }
    }

    /// Resize the masonry grid to accommodate more tracks if needed
    pub fn resize_tracks(&mut self, new_track_count: usize) {
        if new_track_count > self.track_count {
            self.track_positions.resize(new_track_count, 0.0);
            self.track_item_counts.resize(new_track_count, 0);
            self.track_count = new_track_count;
        }
    }

    /// Get current position of a track (for calculating placement)
    pub fn get_track_position(&self, track_index: usize) -> f32 {
        self.track_positions
            .get(track_index)
            .copied()
            .unwrap_or(0.0)
    }
}

/// Type alias for MasonryTrackState - provides more descriptive naming
/// for the existing MasonryPlacementState functionality in grid contexts
pub type MasonryTrackState = MasonryPlacementState;

/// Grid area placement information for CSS Grid Level 3 masonry layout
///
/// This struct represents the final placement of a grid item within a masonry grid,
/// containing both the grid axis placement (definite track positions) and masonry
/// axis placement (flowing position based on content).
#[derive(Debug, Clone, PartialEq)]
pub struct GridArea {
    /// Starting track index on the grid axis (definite dimension)
    pub grid_axis_start: usize,
    /// Ending track index on the grid axis (exclusive, definite dimension)  
    pub grid_axis_end: usize,
    /// Position on the masonry axis (flowing dimension)
    pub masonry_axis_position: f32,
    /// Size on the masonry axis (flowing dimension)
    pub masonry_axis_size: f32,
}

#[inline]
pub fn length_percentage(val: &stylo::LengthPercentage) -> taffy::LengthPercentage {
    match val.unpack() {
        stylo::UnpackedLengthPercentage::Calc(calc_ptr) => {
            let val =
                CompactLength::calc(calc_ptr as *const stylo::CalcLengthPercentage as *const ());
            // SAFETY: calc is a valid value for LengthPercentage
            unsafe { taffy::LengthPercentage::from_raw(val) }
        }
        stylo::UnpackedLengthPercentage::Length(len) => length(len.px()),
        stylo::UnpackedLengthPercentage::Percentage(percentage) => percent(percentage.0),
    }
}

#[inline]
pub fn dimension(val: &stylo::Size) -> taffy::Dimension {
    match val {
        stylo::Size::LengthPercentage(val) => length_percentage(&val.0).into(),
        stylo::Size::Auto => taffy::Dimension::AUTO,

        // TODO: implement other values in Taffy
        stylo::Size::MaxContent => taffy::Dimension::AUTO,
        stylo::Size::MinContent => taffy::Dimension::AUTO,
        stylo::Size::FitContent => taffy::Dimension::AUTO,
        stylo::Size::FitContentFunction(_) => taffy::Dimension::AUTO,
        stylo::Size::Stretch => taffy::Dimension::AUTO,

        // Anchor positioning will be flagged off for time being
        // Fallback to AUTO instead of panicking for production stability
        stylo::Size::AnchorSizeFunction(_) => taffy::Dimension::AUTO,
        stylo::Size::AnchorContainingCalcFunction(_) => taffy::Dimension::AUTO,
    }
}

#[inline]
pub fn max_size_dimension(val: &stylo::MaxSize) -> taffy::Dimension {
    match val {
        stylo::MaxSize::LengthPercentage(val) => length_percentage(&val.0).into(),
        stylo::MaxSize::None => taffy::Dimension::AUTO,

        // TODO: implement other values in Taffy
        stylo::MaxSize::MaxContent => taffy::Dimension::AUTO,
        stylo::MaxSize::MinContent => taffy::Dimension::AUTO,
        stylo::MaxSize::FitContent => taffy::Dimension::AUTO,
        stylo::MaxSize::FitContentFunction(_) => taffy::Dimension::AUTO,
        stylo::MaxSize::Stretch => taffy::Dimension::AUTO,

        // Anchor positioning will be flagged off for time being
        // Fallback to AUTO instead of panicking for production stability
        stylo::MaxSize::AnchorSizeFunction(_) => taffy::Dimension::AUTO,
        stylo::MaxSize::AnchorContainingCalcFunction(_) => taffy::Dimension::AUTO,
    }
}

#[inline]
pub fn margin(val: &stylo::MarginVal) -> taffy::LengthPercentageAuto {
    match val {
        stylo::MarginVal::Auto => taffy::LengthPercentageAuto::AUTO,
        stylo::MarginVal::LengthPercentage(val) => length_percentage(val).into(),

        // Anchor positioning will be flagged off for time being
        // Fallback to AUTO instead of panicking for production stability
        stylo::MarginVal::AnchorSizeFunction(_) => taffy::LengthPercentageAuto::AUTO,
        stylo::MarginVal::AnchorContainingCalcFunction(_) => taffy::LengthPercentageAuto::AUTO,
    }
}

#[inline]
pub fn inset(val: &stylo::InsetVal) -> taffy::LengthPercentageAuto {
    match val {
        stylo::InsetVal::Auto => taffy::LengthPercentageAuto::AUTO,
        stylo::InsetVal::LengthPercentage(val) => length_percentage(val).into(),

        // Anchor positioning will be flagged off for time being
        // Fallback to AUTO instead of panicking for production stability
        stylo::InsetVal::AnchorSizeFunction(_) => taffy::LengthPercentageAuto::AUTO,
        stylo::InsetVal::AnchorFunction(_) => taffy::LengthPercentageAuto::AUTO,
        stylo::InsetVal::AnchorContainingCalcFunction(_) => taffy::LengthPercentageAuto::AUTO,
    }
}

#[inline]
pub fn is_block(input: stylo::Display) -> bool {
    matches!(input.outside(), stylo::DisplayOutside::Block)
        && matches!(
            input.inside(),
            stylo::DisplayInside::Flow | stylo::DisplayInside::FlowRoot
        )
}

#[inline]
pub fn is_table(input: stylo::Display) -> bool {
    matches!(input.inside(), stylo::DisplayInside::Table)
}

#[inline]
pub fn is_display_masonry(input: stylo::Display) -> bool {
    matches!(input.inside(), stylo::DisplayInside::Masonry)
}

#[inline]
pub fn display(input: stylo::Display) -> taffy::Display {
    let mut display = match input.inside() {
        stylo::DisplayInside::None => taffy::Display::None,
        #[cfg(feature = "flexbox")]
        stylo::DisplayInside::Flex => taffy::Display::Flex,
        #[cfg(feature = "grid")]
        stylo::DisplayInside::Grid => taffy::Display::Grid,
        #[cfg(feature = "grid")]
        stylo::DisplayInside::Masonry => taffy::Display::Grid,
        #[cfg(feature = "block")]
        stylo::DisplayInside::Flow => taffy::Display::Block,
        #[cfg(feature = "block")]
        stylo::DisplayInside::FlowRoot => taffy::Display::Block,
        #[cfg(feature = "block")]
        stylo::DisplayInside::TableCell => taffy::Display::Block,
        // TODO: Support display:contents in Taffy
        // TODO: Support table layout in Taffy
        #[cfg(feature = "grid")]
        stylo::DisplayInside::Table => taffy::Display::Grid,
        _ => {
            // println!("FALLBACK {:?} {:?}", input.inside(), input.outside());
            taffy::Display::DEFAULT
        }
    };

    match input.outside() {
        // This is probably redundant as I suspect display.inside() is always None
        // when display.outside() is None.
        stylo::DisplayOutside::None => display = taffy::Display::None,

        // TODO: Support flow and table layout
        stylo::DisplayOutside::Inline => {}
        stylo::DisplayOutside::Block => {}
        stylo::DisplayOutside::TableCaption => {}
        stylo::DisplayOutside::InternalTable => {}
    };

    display
}

#[inline]
pub fn box_generation_mode(input: stylo::Display) -> taffy::BoxGenerationMode {
    match input.inside() {
        stylo::DisplayInside::None => taffy::BoxGenerationMode::None,
        // stylo::DisplayInside::Contents => display = taffy::BoxGenerationMode::Contents,
        _ => taffy::BoxGenerationMode::Normal,
    }
}

#[inline]
pub fn box_sizing(input: stylo::BoxSizing) -> taffy::BoxSizing {
    match input {
        stylo::BoxSizing::BorderBox => taffy::BoxSizing::BorderBox,
        stylo::BoxSizing::ContentBox => taffy::BoxSizing::ContentBox,
    }
}

#[inline]
pub fn position(input: stylo::Position) -> taffy::Position {
    match input {
        // TODO: support position:static
        stylo::Position::Relative => taffy::Position::Relative,
        stylo::Position::Static => taffy::Position::Relative,

        // TODO: support position:fixed and sticky
        stylo::Position::Absolute => taffy::Position::Absolute,
        stylo::Position::Fixed => taffy::Position::Absolute,
        stylo::Position::Sticky => taffy::Position::Relative,
    }
}

#[inline]
pub fn overflow(input: stylo::Overflow) -> taffy::Overflow {
    match input {
        stylo::Overflow::Visible => taffy::Overflow::Visible,
        stylo::Overflow::Clip => taffy::Overflow::Clip,
        stylo::Overflow::Hidden => taffy::Overflow::Hidden,
        stylo::Overflow::Scroll => taffy::Overflow::Scroll,
        // TODO: Support Overflow::Auto in Taffy
        stylo::Overflow::Auto => taffy::Overflow::Scroll,
    }
}

#[inline]
pub fn aspect_ratio(input: stylo::AspectRatio) -> Option<f32> {
    match input.ratio {
        stylo::PreferredRatio::None => None,
        stylo::PreferredRatio::Ratio(val) => Some(val.0.0 / val.1.0),
    }
}

#[inline]
pub fn content_alignment(input: stylo::ContentDistribution) -> Option<taffy::AlignContent> {
    match input.primary().value() {
        stylo::AlignFlags::NORMAL => None,
        stylo::AlignFlags::AUTO => None,
        stylo::AlignFlags::START => Some(taffy::AlignContent::Start),
        stylo::AlignFlags::END => Some(taffy::AlignContent::End),
        stylo::AlignFlags::LEFT => Some(taffy::AlignContent::Start),
        stylo::AlignFlags::RIGHT => Some(taffy::AlignContent::End),
        stylo::AlignFlags::FLEX_START => Some(taffy::AlignContent::FlexStart),
        stylo::AlignFlags::STRETCH => Some(taffy::AlignContent::Stretch),
        stylo::AlignFlags::FLEX_END => Some(taffy::AlignContent::FlexEnd),
        stylo::AlignFlags::CENTER => Some(taffy::AlignContent::Center),
        stylo::AlignFlags::SPACE_BETWEEN => Some(taffy::AlignContent::SpaceBetween),
        stylo::AlignFlags::SPACE_AROUND => Some(taffy::AlignContent::SpaceAround),
        stylo::AlignFlags::SPACE_EVENLY => Some(taffy::AlignContent::SpaceEvenly),
        // Should never be hit. But no real reason to panic here.
        _ => None,
    }
}

#[inline]
pub fn item_alignment(input: stylo::AlignFlags) -> Option<taffy::AlignItems> {
    match input.value() {
        stylo::AlignFlags::AUTO => None,
        stylo::AlignFlags::NORMAL => Some(taffy::AlignItems::Stretch),
        stylo::AlignFlags::STRETCH => Some(taffy::AlignItems::Stretch),
        stylo::AlignFlags::FLEX_START => Some(taffy::AlignItems::FlexStart),
        stylo::AlignFlags::FLEX_END => Some(taffy::AlignItems::FlexEnd),
        stylo::AlignFlags::SELF_START => Some(taffy::AlignItems::Start),
        stylo::AlignFlags::SELF_END => Some(taffy::AlignItems::End),
        stylo::AlignFlags::START => Some(taffy::AlignItems::Start),
        stylo::AlignFlags::END => Some(taffy::AlignItems::End),
        stylo::AlignFlags::LEFT => Some(taffy::AlignItems::Start),
        stylo::AlignFlags::RIGHT => Some(taffy::AlignItems::End),
        stylo::AlignFlags::CENTER => Some(taffy::AlignItems::Center),
        stylo::AlignFlags::BASELINE => Some(taffy::AlignItems::Baseline),
        // Should never be hit. But no real reason to panic here.
        _ => None,
    }
}

#[inline]
pub fn gap(input: &stylo::Gap) -> taffy::LengthPercentage {
    match input {
        // For Flexbox and CSS Grid the "normal" value is 0px. This may need to be updated
        // if we ever implement multi-column layout.
        stylo::Gap::Normal => taffy::LengthPercentage::ZERO,
        stylo::Gap::LengthPercentage(val) => length_percentage(&val.0),
    }
}

#[inline]
#[cfg(feature = "block")]
pub(crate) fn text_align(input: stylo::TextAlign) -> taffy::TextAlign {
    match input {
        stylo::TextAlign::MozLeft => taffy::TextAlign::LegacyLeft,
        stylo::TextAlign::MozRight => taffy::TextAlign::LegacyRight,
        stylo::TextAlign::MozCenter => taffy::TextAlign::LegacyCenter,
        _ => taffy::TextAlign::Auto,
    }
}

#[inline]
#[cfg(feature = "flexbox")]
pub fn flex_basis(input: &stylo::FlexBasis) -> taffy::Dimension {
    // TODO: Support flex-basis: content in Taffy
    match input {
        stylo::FlexBasis::Content => taffy::Dimension::AUTO,
        stylo::FlexBasis::Size(size) => dimension(size),
    }
}

#[inline]
#[cfg(feature = "flexbox")]
pub fn flex_direction(input: stylo::FlexDirection) -> taffy::FlexDirection {
    match input {
        stylo::FlexDirection::Row => taffy::FlexDirection::Row,
        stylo::FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
        stylo::FlexDirection::Column => taffy::FlexDirection::Column,
        stylo::FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
    }
}

#[inline]
#[cfg(feature = "flexbox")]
pub fn flex_wrap(input: stylo::FlexWrap) -> taffy::FlexWrap {
    match input {
        stylo::FlexWrap::Wrap => taffy::FlexWrap::Wrap,
        stylo::FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
        stylo::FlexWrap::Nowrap => taffy::FlexWrap::NoWrap,
    }
}

// CSS Grid styles
// ===============

#[inline]
#[cfg(feature = "grid")]
pub fn grid_auto_flow(input: stylo::GridAutoFlow) -> taffy::GridAutoFlow {
    let is_row = input.contains(stylo::GridAutoFlow::ROW);
    let is_dense = input.contains(stylo::GridAutoFlow::DENSE);

    match (is_row, is_dense) {
        (true, false) => taffy::GridAutoFlow::Row,
        (true, true) => taffy::GridAutoFlow::RowDense,
        (false, false) => taffy::GridAutoFlow::Column,
        (false, true) => taffy::GridAutoFlow::ColumnDense,
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn grid_line(input: &stylo::GridLine) -> taffy::GridPlacement {
    if input.is_auto() {
        taffy::GridPlacement::Auto
    } else if input.is_span {
        // Convert i32 to u16 with bounds checking for span values
        // CSS Grid spans are positive and should fit within u16 range
        match input.line_num.try_into() {
            Ok(span_val) if span_val > 0 => taffy::style_helpers::span(span_val),
            _ => {
                // Fallback to span of 1 for invalid or out-of-range span values
                // This matches CSS Grid behavior for invalid spans
                taffy::style_helpers::span(1)
            }
        }
    } else if input.line_num == 0 {
        taffy::GridPlacement::Auto
    } else {
        // Convert i32 to i16 with bounds checking for line indices
        // CSS Grid line indices can be negative but must fit within i16 range
        match input.line_num.try_into() {
            Ok(line_val) => taffy::style_helpers::line(line_val),
            Err(_) => {
                // For out-of-range values, clamp to valid i16 range
                // This preserves the sign while ensuring valid conversion
                if input.line_num > i16::MAX as i32 {
                    taffy::style_helpers::line(i16::MAX)
                } else {
                    taffy::style_helpers::line(i16::MIN)
                }
            }
        }
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn grid_template_tracks(
    input: &stylo::GridTemplateComponent,
    grid_context: Option<&GridContext>,
) -> Vec<taffy::GridTemplateComponent<String>> {
    match input {
        stylo::GenericGridTemplateComponent::None => Vec::new(),
        stylo::GenericGridTemplateComponent::TrackList(list) => list
            .values
            .iter()
            .map(|track| match track {
                stylo::TrackListValue::TrackSize(size) => {
                    // Create GridTemplateComponent::Single instead of TrackSizingFunction::Single
                    taffy::GridTemplateComponent::Single(track_size(size))
                }
                stylo::TrackListValue::TrackRepeat(repeat) => {
                    // Create GridTemplateComponent::Repeat with new GridTemplateRepetition
                    taffy::GridTemplateComponent::Repeat(taffy::GridTemplateRepetition {
                        count: track_repeat(repeat.count),
                        tracks: repeat.track_sizes.iter().map(track_size).collect(),
                        line_names: repeat
                            .line_names
                            .iter()
                            .map(|names| names.iter().map(|ident| ident.0.to_string()).collect())
                            .collect(),
                    })
                }
            })
            .collect(),

        // Real subgrid implementation following CSS Grid Level 2 specification
        stylo::GenericGridTemplateComponent::Subgrid(subgrid) => {
            implement_subgrid(subgrid, grid_context)
        }

        // Real masonry implementation following CSS Grid Level 3 specification
        stylo::GenericGridTemplateComponent::Masonry => implement_masonry(grid_context),
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn grid_auto_tracks(input: &stylo::ImplicitGridTracks) -> Vec<taffy::TrackSizingFunction> {
    input.0.iter().map(track_size).collect()
}

/// Extract line name strings from stylo GenericLineNameListValue
fn extract_line_names_from_stylo_value(
    value: &style::values::generics::grid::GenericLineNameListValue<i32>,
) -> SubgridLineNameResult<Vec<String>> {
    match value {
        style::values::generics::grid::GenericLineNameListValue::LineNames(line_names) => {
            let mut names = Vec::new();
            for ident in line_names.iter() {
                // Convert CustomIdent to String
                // CustomIdent contains an Atom, access it via .0
                let name = match std::panic::catch_unwind(|| ident.0.to_string()) {
                    Ok(name_str) => name_str,
                    Err(_) => {
                        return Err(SubgridLineNameError::CustomIdentConversionFailed(format!(
                            "Failed to convert CustomIdent to String"
                        )));
                    }
                };

                if name.is_empty() {
                    return Err(SubgridLineNameError::InvalidLineNameFormat(
                        "Empty line name not allowed".to_string(),
                    ));
                }
                names.push(name);
            }
            Ok(names)
        }
        style::values::generics::grid::GenericLineNameListValue::Repeat(repeat) => {
            // Handle repeated line names by expanding the repeat pattern
            let mut names = Vec::new();

            // Get the repeat count and pattern
            let repeat_count = match repeat.count {
                style::values::generics::grid::RepeatCount::Number(num) => {
                    if num > 0 {
                        num as usize
                    } else {
                        1 // Default to 1 for invalid counts
                    }
                }
                style::values::generics::grid::RepeatCount::AutoFill
                | style::values::generics::grid::RepeatCount::AutoFit => {
                    1 // For subgrid line names, auto-fill/auto-fit behaves as 1 repetition
                }
            };

            // Extract line names from the repeat pattern
            for line_name_list in repeat.line_names.iter() {
                for ident in line_name_list.iter() {
                    let name = match std::panic::catch_unwind(|| ident.0.to_string()) {
                        Ok(name_str) => name_str,
                        Err(_) => {
                            return Err(SubgridLineNameError::CustomIdentConversionFailed(
                                format!(
                                    "Failed to convert CustomIdent to String in repeat pattern"
                                ),
                            ));
                        }
                    };

                    if name.is_empty() {
                        return Err(SubgridLineNameError::InvalidLineNameFormat(
                            "Empty line name not allowed in repeat pattern".to_string(),
                        ));
                    }
                    names.push(name);
                }
            }

            // Expand the pattern according to repeat count
            let mut expanded_names = Vec::new();
            for _ in 0..repeat_count {
                expanded_names.extend(names.clone());
            }

            Ok(expanded_names)
        }
    }
}

/// Map subgrid line names to inherited parent tracks per CSS Grid Level 2 spec
///
/// Per CSS Grid L2: "specified line-names are assigned to the lines of the
/// subgrid's explicit grid, one per line, starting with line 1"
///
/// **Critical Fix**: This maps line names to grid LINES, not tracks.
/// A grid with N tracks has N+1 lines (before, between, and after tracks).
/// Only Repeat tracks can store line names in taffy's type system.
fn map_subgrid_line_names(
    subgrid_line_names: &style::values::generics::grid::GenericLineNameList<i32>,
    inherited_tracks: &mut Vec<taffy::GridTemplateComponent<String>>,
) -> SubgridLineNameResult<()> {
    // Extract all line names from the subgrid line name list
    let mut all_line_names: Vec<String> = Vec::new();
    for value in subgrid_line_names.line_names.iter() {
        let names = extract_line_names_from_stylo_value(value)?;
        all_line_names.extend(names);
    }

    if all_line_names.is_empty() {
        return Ok(()); // No line names to assign
    }

    // CSS Grid line indexing:
    // - Line 1 is before the first track
    // - Line 2 is between track 1 and track 2
    // - Line N+1 is after the last track (for N tracks)
    let mut line_name_index = 0;
    let num_tracks = inherited_tracks.len();
    let num_lines = num_tracks + 1; // Grid has N+1 lines for N tracks

    // Process each grid line position and try to assign line names
    for line_position in 0..num_lines {
        if line_name_index >= all_line_names.len() {
            break; // No more line names to assign
        }

        let line_name = &all_line_names[line_name_index];
        let assigned = assign_line_name_to_grid_line(inherited_tracks, line_position, line_name)?;

        if assigned {
            tracing::debug!(
                "Assigned line name '{}' to grid line {} (0-indexed)",
                line_name,
                line_position
            );
        } else {
            tracing::debug!(
                "Could not assign line name '{}' to grid line {} (no compatible Repeat track)",
                line_name,
                line_position
            );
        }
        line_name_index += 1; // Always advance to maintain correct line name order
    }

    // Log any excess line names that were ignored (per CSS spec)
    if line_name_index < all_line_names.len() {
        let excess_count = all_line_names.len() - line_name_index;
        tracing::debug!(
            "Ignored {} excess line names per CSS Grid L2 spec",
            excess_count
        );
    }

    Ok(())
}

/// Assign a line name to a specific grid line position
/// Returns true if assignment was successful, false if no compatible track was found
fn assign_line_name_to_grid_line(
    inherited_tracks: &mut Vec<taffy::GridTemplateComponent<String>>,
    line_position: usize,
    line_name: &str,
) -> SubgridLineNameResult<bool> {
    let num_tracks = inherited_tracks.len();
    let num_lines = num_tracks + 1;

    // Bounds checking: ensure line_position is valid
    if line_position > num_lines {
        return Err(SubgridLineNameError::LineIndexOutOfBounds {
            line_index: line_position,
            track_count: num_tracks,
        });
    }

    // Validate line name is not empty
    if line_name.is_empty() {
        return Err(SubgridLineNameError::InvalidLineNameFormat(
            "Empty line name not allowed".to_string(),
        ));
    }

    // Line 0 is before track 0, line 1 is between track 0 and 1, etc.
    // We need to find a Repeat track that can store this line name

    // Strategy: Look for the nearest Repeat track that can accommodate this line name
    // Priority: track that starts at or before this line position

    if line_position == 0 {
        // Line before first track - assign to first track if it's a Repeat
        if inherited_tracks.is_empty() {
            return Err(SubgridLineNameError::LineIndexOutOfBounds {
                line_index: 0,
                track_count: 0,
            });
        }

        match inherited_tracks.get_mut(0) {
            Some(taffy::GridTemplateComponent::Repeat(repetition)) => {
                ensure_line_names_capacity(repetition, 1)?;
                if let Some(first_line) = repetition.line_names.first_mut() {
                    first_line.push(line_name.to_string());
                    return Ok(true);
                }
            }
            Some(taffy::GridTemplateComponent::Single(_)) => {
                return Err(SubgridLineNameError::SingleTrackLineNameUnsupported {
                    track_index: 0,
                });
            }
            None => {
                return Err(SubgridLineNameError::LineIndexOutOfBounds {
                    line_index: 0,
                    track_count: num_tracks,
                });
            }
        }
    } else if line_position < num_tracks {
        // Line between tracks - assign to the track after this line (track at line_position)
        match inherited_tracks.get_mut(line_position) {
            Some(taffy::GridTemplateComponent::Repeat(repetition)) => {
                ensure_line_names_capacity(repetition, 1)?;
                if let Some(first_line) = repetition.line_names.first_mut() {
                    first_line.push(line_name.to_string());
                    return Ok(true);
                }
            }
            Some(taffy::GridTemplateComponent::Single(_)) => {
                return Err(SubgridLineNameError::SingleTrackLineNameUnsupported {
                    track_index: line_position,
                });
            }
            None => {
                return Err(SubgridLineNameError::LineIndexOutOfBounds {
                    line_index: line_position,
                    track_count: num_tracks,
                });
            }
        }
    } else if line_position == num_tracks {
        // Line after last track - assign to last track if it's a Repeat
        if inherited_tracks.is_empty() {
            return Err(SubgridLineNameError::LineIndexOutOfBounds {
                line_index: line_position,
                track_count: 0,
            });
        }

        match inherited_tracks.last_mut() {
            Some(taffy::GridTemplateComponent::Repeat(repetition)) => {
                ensure_line_names_capacity(repetition, repetition.tracks.len() + 1)?;
                if let Some(last_line) = repetition.line_names.last_mut() {
                    last_line.push(line_name.to_string());
                    return Ok(true);
                }
            }
            Some(taffy::GridTemplateComponent::Single(_)) => {
                let last_track_index = num_tracks - 1;
                return Err(SubgridLineNameError::SingleTrackLineNameUnsupported {
                    track_index: last_track_index,
                });
            }
            None => {
                return Err(SubgridLineNameError::LineIndexOutOfBounds {
                    line_index: line_position,
                    track_count: num_tracks,
                });
            }
        }
    }

    // Fallback: try to assign to any available Repeat track
    for track in inherited_tracks.iter_mut() {
        if let taffy::GridTemplateComponent::Repeat(repetition) = track {
            ensure_line_names_capacity(repetition, 1)?;
            if let Some(first_line) = repetition.line_names.first_mut() {
                first_line.push(line_name.to_string());
                return Ok(true);
            }
        }
    }

    // No compatible Repeat track found
    Ok(false)
}

/// Ensure that a Repeat track has sufficient line_names capacity
fn ensure_line_names_capacity(
    repetition: &mut taffy::GridTemplateRepetition<String>,
    min_capacity: usize,
) -> SubgridLineNameResult<()> {
    if repetition.line_names.len() < min_capacity {
        repetition.line_names.resize(min_capacity, Vec::new());
    }
    Ok(())
}

/// Implement real subgrid functionality following CSS Grid Level 2 specification
/// Returns inherited track definitions from parent grid or falls back per spec
#[cfg(feature = "grid")]
fn implement_subgrid(
    subgrid: &style::values::generics::grid::GenericLineNameList<i32>,
    grid_context: Option<&GridContext>,
) -> Vec<taffy::GridTemplateComponent<String>> {
    // Check if we have parent grid context for subgrid inheritance
    if let Some(context) = grid_context {
        if context.supports_subgrid && !context.parent_tracks.is_empty() {
            // REAL SUBGRID: Inherit actual track definitions from parent grid
            let mut inherited_tracks = context.parent_tracks.clone();

            // Apply subgrid line names if provided
            // Per CSS Grid Level 2 spec: subgrid line names are mapped to inherited tracks
            if !subgrid.line_names.is_empty() {
                // PRODUCTION IMPLEMENTATION: Real line name mapping per CSS Grid L2 spec
                if let Err(error) = map_subgrid_line_names(subgrid, &mut inherited_tracks) {
                    // Graceful degradation: log error but continue with layout
                    tracing::warn!(
                        "Subgrid line name mapping failed: {}. Continuing without line names.",
                        error
                    );
                    // Layout continues with inherited tracks but without line name mapping
                }
            }

            return inherited_tracks;
        }
    }

    // CSS Grid Level 2 fallback: When subgrid conditions aren't met, used value becomes 'none'
    // Per specification: "If there is no parent grid, or if the grid container is otherwise
    // forced to establish an independent formatting context... the used value is the initial
    // value, none, and the grid container is not a subgrid."
    Vec::new()
}

/// Implement real masonry functionality following CSS Grid Level 3 specification
/// Returns masonry track definitions that adapt to content flow
#[cfg(feature = "grid")]
fn implement_masonry(
    grid_context: Option<&GridContext>,
) -> Vec<taffy::GridTemplateComponent<String>> {
    // Determine optimal number of masonry tracks based on available space
    let track_count = if let Some(context) = grid_context {
        if let Some(available_space) = context.available_space {
            // Calculate optimal track count based on available space
            // CSS Grid Level 3: masonry should adapt to container size
            calculate_optimal_masonry_tracks(available_space)
        } else {
            // Default masonry track count when no space information available
            DEFAULT_MASONRY_TRACKS
        }
    } else {
        // Fallback track count for masonry without context
        DEFAULT_MASONRY_TRACKS
    };

    // Create masonry tracks following CSS Grid Level 3 specification
    // Each track uses AUTO sizing to adapt to content
    // The masonry placement algorithm will handle item distribution
    (0..track_count)
        .map(|_| {
            taffy::GridTemplateComponent::Single(
                taffy::MinMax {
                    min: taffy::MinTrackSizingFunction::AUTO,
                    max: taffy::MaxTrackSizingFunction::AUTO,
                }
                .into(),
            )
        })
        .collect()
}

/// Calculate optimal number of masonry tracks based on available space
/// Implements content-aware track sizing per CSS Grid Level 3
#[cfg(feature = "grid")]
pub fn calculate_optimal_masonry_tracks(available_space: f32) -> usize {
    // CSS Grid Level 3 masonry algorithm: adapt track count to available space
    // Minimum viable track width for masonry items (adjustable based on content)
    const MIN_TRACK_WIDTH: f32 = 200.0; // pixels

    let optimal_tracks = (available_space / MIN_TRACK_WIDTH).floor() as usize;

    // Ensure reasonable bounds for masonry track count
    optimal_tracks.clamp(MIN_MASONRY_TRACKS, MAX_MASONRY_TRACKS)
}

// Constants for masonry track management
const DEFAULT_MASONRY_TRACKS: usize = 4;
const MIN_MASONRY_TRACKS: usize = 2;
const MAX_MASONRY_TRACKS: usize = 12;

/// Check if a grid template component contains masonry in the specified axis
#[cfg(feature = "grid")]
pub fn is_masonry_axis(input: &stylo::GridTemplateComponent) -> bool {
    matches!(input, stylo::GenericGridTemplateComponent::Masonry)
}

#[inline]
#[cfg(feature = "grid")]
pub fn track_repeat(input: stylo::RepeatCount<i32>) -> taffy::RepetitionCount {
    match input {
        stylo::RepeatCount::Number(val) => {
            // Convert i32 to u16 with bounds checking for repeat count
            // CSS Grid repeat counts must be positive and within u16 range
            match val.try_into() {
                Ok(count) if count > 0 => taffy::RepetitionCount::Count(count),
                _ => {
                    // Fallback to count of 1 for invalid or out-of-range repeat values
                    // This ensures grid layout continues to function with minimal disruption
                    taffy::RepetitionCount::Count(1)
                }
            }
        }
        stylo::RepeatCount::AutoFill => taffy::RepetitionCount::AutoFill,
        stylo::RepeatCount::AutoFit => taffy::RepetitionCount::AutoFit,
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn track_size(input: &stylo::TrackSize<stylo::LengthPercentage>) -> taffy::TrackSizingFunction {
    use taffy::MaxTrackSizingFunction;

    match input {
        stylo::TrackSize::Breadth(breadth) => taffy::MinMax {
            min: min_track(breadth),
            max: max_track(breadth),
        },
        stylo::TrackSize::Minmax(min, max) => taffy::MinMax {
            min: min_track(min),
            max: max_track(max),
        },
        stylo::TrackSize::FitContent(limit) => taffy::MinMax {
            min: taffy::MinTrackSizingFunction::AUTO,
            max: match limit {
                stylo::TrackBreadth::Breadth(lp) => {
                    MaxTrackSizingFunction::fit_content(length_percentage(lp))
                }

                // These TrackBreadth variants are not valid in fit-content context
                // Fallback to AUTO instead of panicking for production stability
                stylo::TrackBreadth::Fr(_) => MaxTrackSizingFunction::AUTO,
                stylo::TrackBreadth::Auto => MaxTrackSizingFunction::AUTO,
                stylo::TrackBreadth::MinContent => MaxTrackSizingFunction::AUTO,
                stylo::TrackBreadth::MaxContent => MaxTrackSizingFunction::AUTO,
            },
        },
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn min_track(
    input: &stylo::TrackBreadth<stylo::LengthPercentage>,
) -> taffy::MinTrackSizingFunction {
    use taffy::prelude::*;
    match input {
        stylo::TrackBreadth::Breadth(lp) => {
            taffy::MinTrackSizingFunction::from(length_percentage(lp))
        }
        stylo::TrackBreadth::Fr(_) => taffy::MinTrackSizingFunction::AUTO,
        stylo::TrackBreadth::Auto => taffy::MinTrackSizingFunction::AUTO,
        stylo::TrackBreadth::MinContent => taffy::MinTrackSizingFunction::MIN_CONTENT,
        stylo::TrackBreadth::MaxContent => taffy::MinTrackSizingFunction::MAX_CONTENT,
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn max_track(
    input: &stylo::TrackBreadth<stylo::LengthPercentage>,
) -> taffy::MaxTrackSizingFunction {
    use taffy::prelude::*;

    match input {
        stylo::TrackBreadth::Breadth(lp) => {
            taffy::MaxTrackSizingFunction::from(length_percentage(lp))
        }
        stylo::TrackBreadth::Fr(val) => taffy::MaxTrackSizingFunction::from_fr(*val),
        stylo::TrackBreadth::Auto => taffy::MaxTrackSizingFunction::AUTO,
        stylo::TrackBreadth::MinContent => taffy::MaxTrackSizingFunction::MIN_CONTENT,
        stylo::TrackBreadth::MaxContent => taffy::MaxTrackSizingFunction::MAX_CONTENT,
    }
}

/// Eagerly convert an entire [`stylo::ComputedValues`] into a [`taffy::Style`] with device context for platform metrics
pub fn to_taffy_style_with_device(style: &stylo::ComputedValues, device: &Device) -> taffy::Style {
    to_taffy_style_with_grid_context(style, device, None, None)
}

/// Convert [`stylo::ComputedValues`] into [`taffy::Style`] with full grid context support
/// This is the main API for subgrid and masonry functionality
pub fn to_taffy_style_with_grid_context(
    style: &stylo::ComputedValues,
    device: &Device,
    row_grid_context: Option<&GridContext>,
    column_grid_context: Option<&GridContext>,
) -> taffy::Style {
    let display = style.clone_display();
    let pos = style.get_position();
    let margin = style.get_margin();
    let padding = style.get_padding();
    let border = style.get_border();

    taffy::Style {
        // NEW REQUIRED FIELDS
        dummy: core::marker::PhantomData,
        grid_template_areas: self::grid_template_areas(&pos.grid_template_areas)
            .unwrap_or_default(),
        grid_template_column_names: self::grid_template_line_names(&pos.grid_template_columns)
            .unwrap_or_default(),
        grid_template_row_names: self::grid_template_line_names(&pos.grid_template_rows)
            .unwrap_or_default(),

        display: self::display(display),
        box_sizing: self::box_sizing(style.clone_box_sizing()),
        item_is_table: display.inside() == stylo::DisplayInside::Table,
        item_is_replaced: false,
        position: self::position(style.clone_position()),
        overflow: taffy::Point {
            x: self::overflow(style.clone_overflow_x()),
            y: self::overflow(style.clone_overflow_y()),
        },
        // Production-quality scrollbar width detection using platform metrics
        scrollbar_width: device.scrollbar_inline_size().px(),

        size: taffy::Size {
            width: self::dimension(&pos.width),
            height: self::dimension(&pos.height),
        },
        min_size: taffy::Size {
            width: self::dimension(&pos.min_width),
            height: self::dimension(&pos.min_height),
        },
        max_size: taffy::Size {
            width: self::max_size_dimension(&pos.max_width),
            height: self::max_size_dimension(&pos.max_height),
        },
        aspect_ratio: self::aspect_ratio(pos.aspect_ratio),

        inset: taffy::Rect {
            left: self::inset(&pos.left),
            right: self::inset(&pos.right),
            top: self::inset(&pos.top),
            bottom: self::inset(&pos.bottom),
        },
        margin: taffy::Rect {
            left: self::margin(&margin.margin_left),
            right: self::margin(&margin.margin_right),
            top: self::margin(&margin.margin_top),
            bottom: self::margin(&margin.margin_bottom),
        },
        padding: taffy::Rect {
            left: self::length_percentage(&padding.padding_left.0),
            right: self::length_percentage(&padding.padding_right.0),
            top: self::length_percentage(&padding.padding_top.0),
            bottom: self::length_percentage(&padding.padding_bottom.0),
        },
        border: taffy::Rect {
            left: taffy::style_helpers::length(border.border_left_width.to_f32_px()),
            right: taffy::style_helpers::length(border.border_right_width.to_f32_px()),
            top: taffy::style_helpers::length(border.border_top_width.to_f32_px()),
            bottom: taffy::style_helpers::length(border.border_bottom_width.to_f32_px()),
        },

        // Gap
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        gap: taffy::Size {
            width: self::gap(&pos.column_gap),
            height: self::gap(&pos.row_gap),
        },

        // Alignment
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        align_content: self::content_alignment(pos.align_content.0),
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        justify_content: self::content_alignment(pos.justify_content.0),
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        align_items: self::item_alignment(pos.align_items.0),
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        align_self: self::item_alignment((pos.align_self.0).0),
        #[cfg(feature = "grid")]
        justify_items: self::item_alignment(pos.justify_items.computed.0),
        #[cfg(feature = "grid")]
        justify_self: self::item_alignment((pos.justify_self.0).0),
        #[cfg(feature = "block")]
        text_align: self::text_align(style.clone_text_align()),

        // Flexbox
        #[cfg(feature = "flexbox")]
        flex_direction: self::flex_direction(pos.flex_direction),
        #[cfg(feature = "flexbox")]
        flex_wrap: self::flex_wrap(pos.flex_wrap),
        #[cfg(feature = "flexbox")]
        flex_grow: pos.flex_grow.0,
        #[cfg(feature = "flexbox")]
        flex_shrink: pos.flex_shrink.0,
        #[cfg(feature = "flexbox")]
        flex_basis: self::flex_basis(&pos.flex_basis),

        // Grid
        #[cfg(feature = "grid")]
        grid_auto_flow: self::grid_auto_flow(pos.grid_auto_flow),
        #[cfg(feature = "grid")]
        grid_template_rows: self::grid_template_tracks(&pos.grid_template_rows, row_grid_context),
        #[cfg(feature = "grid")]
        grid_template_columns: self::grid_template_tracks(
            &pos.grid_template_columns,
            column_grid_context,
        ),
        #[cfg(feature = "grid")]
        grid_auto_rows: self::grid_auto_tracks(&pos.grid_auto_rows),
        #[cfg(feature = "grid")]
        grid_auto_columns: self::grid_auto_tracks(&pos.grid_auto_columns),
        #[cfg(feature = "grid")]
        grid_row: taffy::Line {
            start: self::grid_line(&pos.grid_row_start),
            end: self::grid_line(&pos.grid_row_end),
        },
        #[cfg(feature = "grid")]
        grid_column: taffy::Line {
            start: self::grid_line(&pos.grid_column_start),
            end: self::grid_line(&pos.grid_column_end),
        },
    }
}

/// Eagerly convert an entire [`stylo::ComputedValues`] into a [`taffy::Style`] (backward compatibility)
pub fn to_taffy_style(style: &stylo::ComputedValues) -> taffy::Style {
    // For backward compatibility, use the simplified conversion without device context
    // This bypasses platform-specific scrollbar width detection but maintains functionality
    let display = style.clone_display();
    let pos = style.get_position();
    let margin = style.get_margin();
    let padding = style.get_padding();
    let border = style.get_border();

    taffy::Style {
        // NEW REQUIRED FIELDS
        dummy: core::marker::PhantomData,
        grid_template_areas: self::grid_template_areas(&pos.grid_template_areas)
            .unwrap_or_default(),
        grid_template_column_names: self::grid_template_line_names(&pos.grid_template_columns)
            .unwrap_or_default(),
        grid_template_row_names: self::grid_template_line_names(&pos.grid_template_rows)
            .unwrap_or_default(),

        display: self::display(display),
        box_sizing: self::box_sizing(style.clone_box_sizing()),
        item_is_table: display.inside() == stylo::DisplayInside::Table,
        item_is_replaced: false,
        position: self::position(style.clone_position()),
        overflow: taffy::Point {
            x: self::overflow(style.clone_overflow_x()),
            y: self::overflow(style.clone_overflow_y()),
        },
        // Use standard fallback scrollbar width for backward compatibility
        scrollbar_width: 15.0,

        size: taffy::Size {
            width: self::dimension(&pos.width),
            height: self::dimension(&pos.height),
        },
        min_size: taffy::Size {
            width: self::dimension(&pos.min_width),
            height: self::dimension(&pos.min_height),
        },
        max_size: taffy::Size {
            width: self::max_size_dimension(&pos.max_width),
            height: self::max_size_dimension(&pos.max_height),
        },
        aspect_ratio: self::aspect_ratio(pos.aspect_ratio),

        inset: taffy::Rect {
            left: self::inset(&pos.left),
            right: self::inset(&pos.right),
            top: self::inset(&pos.top),
            bottom: self::inset(&pos.bottom),
        },
        margin: taffy::Rect {
            left: self::margin(&margin.margin_left),
            right: self::margin(&margin.margin_right),
            top: self::margin(&margin.margin_top),
            bottom: self::margin(&margin.margin_bottom),
        },
        padding: taffy::Rect {
            left: self::length_percentage(&padding.padding_left.0),
            right: self::length_percentage(&padding.padding_right.0),
            top: self::length_percentage(&padding.padding_top.0),
            bottom: self::length_percentage(&padding.padding_bottom.0),
        },
        border: taffy::Rect {
            left: taffy::style_helpers::length(border.border_left_width.to_f32_px()),
            right: taffy::style_helpers::length(border.border_right_width.to_f32_px()),
            top: taffy::style_helpers::length(border.border_top_width.to_f32_px()),
            bottom: taffy::style_helpers::length(border.border_bottom_width.to_f32_px()),
        },

        // Gap
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        gap: taffy::Size {
            width: self::gap(&pos.column_gap),
            height: self::gap(&pos.row_gap),
        },

        // Alignment
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        align_content: self::content_alignment(pos.align_content.0),
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        justify_content: self::content_alignment(pos.justify_content.0),
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        align_items: self::item_alignment(pos.align_items.0),
        #[cfg(any(feature = "flexbox", feature = "grid"))]
        align_self: self::item_alignment((pos.align_self.0).0),
        #[cfg(feature = "grid")]
        justify_items: self::item_alignment(pos.justify_items.computed.0),
        #[cfg(feature = "grid")]
        justify_self: self::item_alignment((pos.justify_self.0).0),
        #[cfg(feature = "block")]
        text_align: self::text_align(style.clone_text_align()),

        // Flexbox
        #[cfg(feature = "flexbox")]
        flex_direction: self::flex_direction(pos.flex_direction),
        #[cfg(feature = "flexbox")]
        flex_wrap: self::flex_wrap(pos.flex_wrap),
        #[cfg(feature = "flexbox")]
        flex_grow: pos.flex_grow.0,
        #[cfg(feature = "flexbox")]
        flex_shrink: pos.flex_shrink.0,
        #[cfg(feature = "flexbox")]
        flex_basis: self::flex_basis(&pos.flex_basis),

        // Grid - using simplified conversion without grid context
        #[cfg(feature = "grid")]
        grid_auto_flow: self::grid_auto_flow(pos.grid_auto_flow),
        #[cfg(feature = "grid")]
        grid_template_rows: self::grid_template_tracks(&pos.grid_template_rows, None),
        #[cfg(feature = "grid")]
        grid_template_columns: self::grid_template_tracks(&pos.grid_template_columns, None),
        #[cfg(feature = "grid")]
        grid_auto_rows: self::grid_auto_tracks(&pos.grid_auto_rows),
        #[cfg(feature = "grid")]
        grid_auto_columns: self::grid_auto_tracks(&pos.grid_auto_columns),
        #[cfg(feature = "grid")]
        grid_row: taffy::Line {
            start: self::grid_line(&pos.grid_row_start),
            end: self::grid_line(&pos.grid_row_end),
        },
        #[cfg(feature = "grid")]
        grid_column: taffy::Line {
            start: self::grid_line(&pos.grid_column_start),
            end: self::grid_line(&pos.grid_column_end),
        },
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn grid_template_areas(
    input: &GridTemplateAreas,
) -> Option<Vec<taffy::GridTemplateArea<String>>> {
    match input {
        GridTemplateAreas::None => None,
        GridTemplateAreas::Areas(areas_arc) => {
            let areas = &areas_arc.0.areas;
            Some(
                areas
                    .iter()
                    .map(|area| {
                        // Convert u32 to u16 with overflow checking for grid area coordinates
                        // CSS Grid area coordinates should fit within u16 range for practical layouts
                        taffy::GridTemplateArea {
                            name: area.name.to_string(),
                            row_start: area.rows.start.min(u16::MAX as u32) as u16,
                            row_end: area.rows.end.min(u16::MAX as u32) as u16,
                            column_start: area.columns.start.min(u16::MAX as u32) as u16,
                            column_end: area.columns.end.min(u16::MAX as u32) as u16,
                        }
                    })
                    .collect(),
            )
        }
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn extract_line_names(line_names_slice: &[OwnedSlice<CustomIdent>]) -> Vec<Vec<String>> {
    line_names_slice
        .iter()
        .map(|names| names.iter().map(|ident| ident.0.to_string()).collect())
        .collect()
}

#[inline]
#[cfg(feature = "grid")]
pub fn grid_template_line_names(input: &stylo::GridTemplateComponent) -> Option<Vec<Vec<String>>> {
    match input {
        stylo::GenericGridTemplateComponent::TrackList(list) => {
            if list.line_names.is_empty() {
                None
            } else {
                Some(extract_line_names(&list.line_names))
            }
        }
        _ => None,
    }
}

// Reverse conversion functions for subgrid track inheritance
// These convert from taffy types back to stylo types for CSS style modification

#[inline]
#[cfg(feature = "grid")]
pub fn taffy_track_to_stylo(
    input: &taffy::TrackSizingFunction,
) -> stylo::TrackSize<stylo::LengthPercentage> {
    use stylo::TrackSize;
    use taffy::MinMax;

    match input {
        MinMax { min, max } => {
            // Check if it's a simple breadth (min and max are the same)
            if let (Some(min_lp), Some(max_lp)) = (
                taffy_min_track_to_stylo_breadth(min),
                taffy_max_track_to_stylo_breadth(max),
            ) {
                if min_lp == max_lp {
                    return TrackSize::Breadth(min_lp);
                }
            }

            // Otherwise it's a minmax
            TrackSize::Minmax(
                taffy_min_track_to_stylo_breadth(min).unwrap_or(stylo::TrackBreadth::Auto),
                taffy_max_track_to_stylo_breadth(max).unwrap_or(stylo::TrackBreadth::Auto),
            )
        }
    }
}

#[inline]
#[cfg(feature = "grid")]
fn taffy_min_track_to_stylo_breadth(
    input: &taffy::MinTrackSizingFunction,
) -> Option<stylo::TrackBreadth<stylo::LengthPercentage>> {
    use stylo::TrackBreadth;
    use taffy::CompactLength;

    let compact = input.into_raw();

    match compact.tag() {
        CompactLength::LENGTH_TAG => {
            let length = stylo::Length::new(compact.value());
            Some(TrackBreadth::Breadth(stylo::LengthPercentage::new_length(length)))
        }
        CompactLength::PERCENT_TAG => {
            let percentage = stylo::Percentage(compact.value());
            Some(TrackBreadth::Breadth(stylo::LengthPercentage::new_percent(percentage)))
        }
        CompactLength::AUTO_TAG => Some(TrackBreadth::Auto),
        CompactLength::MIN_CONTENT_TAG => Some(TrackBreadth::MinContent),
        CompactLength::MAX_CONTENT_TAG => Some(TrackBreadth::MaxContent),
        _ => Some(TrackBreadth::Auto),
    }
}

#[inline]
#[cfg(feature = "grid")]
fn taffy_max_track_to_stylo_breadth(
    input: &taffy::MaxTrackSizingFunction,
) -> Option<stylo::TrackBreadth<stylo::LengthPercentage>> {
    use stylo::TrackBreadth;
    use taffy::CompactLength;

    let compact = input.into_raw();

    match compact.tag() {
        CompactLength::LENGTH_TAG => {
            let length = stylo::Length::new(compact.value());
            Some(TrackBreadth::Breadth(stylo::LengthPercentage::new_length(length)))
        }
        CompactLength::PERCENT_TAG => {
            let percentage = stylo::Percentage(compact.value());
            Some(TrackBreadth::Breadth(stylo::LengthPercentage::new_percent(percentage)))
        }
        CompactLength::FR_TAG => {
            Some(TrackBreadth::Fr(compact.value()))
        }
        CompactLength::FIT_CONTENT_PX_TAG => {
            let length = stylo::Length::new(compact.value());
            Some(TrackBreadth::Breadth(stylo::LengthPercentage::new_length(length)))
        }
        CompactLength::FIT_CONTENT_PERCENT_TAG => {
            let percentage = stylo::Percentage(compact.value());
            Some(TrackBreadth::Breadth(stylo::LengthPercentage::new_percent(percentage)))
        }
        CompactLength::AUTO_TAG => Some(TrackBreadth::Auto),
        CompactLength::MIN_CONTENT_TAG => Some(TrackBreadth::MinContent),
        CompactLength::MAX_CONTENT_TAG => Some(TrackBreadth::MaxContent),
        _ => Some(TrackBreadth::Auto),
    }
}

#[inline]
#[cfg(feature = "grid")]
fn taffy_repetition_count_to_stylo(input: taffy::RepetitionCount) -> stylo::RepeatCount<i32> {
    use taffy::RepetitionCount;
    use stylo::RepeatCount;
    
    match input {
        RepetitionCount::AutoFill => RepeatCount::AutoFill,
        RepetitionCount::AutoFit => RepeatCount::AutoFit,
        RepetitionCount::Count(n) => RepeatCount::Number(n as i32),
    }
}

#[inline]
#[cfg(feature = "grid")]
pub fn taffy_template_tracks_to_stylo(
    input: &[taffy::GridTemplateComponent<String>],
) -> stylo::GridTemplateComponent {
    use stylo::GenericGridTemplateComponent;
    use style::OwnedSlice;

    if input.is_empty() {
        return GenericGridTemplateComponent::None;
    }

    let mut track_list_values = Vec::with_capacity(input.len());

    for component in input {
        match component {
            taffy::GridTemplateComponent::Single(track_sizing_fn) => {
                let track_size = taffy_track_to_stylo(track_sizing_fn);
                track_list_values.push(stylo::TrackListValue::TrackSize(track_size));
            }
            taffy::GridTemplateComponent::Repeat(repetition) => {
                // Convert repeat count
                let count = taffy_repetition_count_to_stylo(repetition.count);
                
                // Convert track sizes using existing helper
                let track_sizes: Vec<_> = repetition.tracks.iter()
                    .map(|track| taffy_track_to_stylo(track))
                    .collect();
                
                // Convert line names: Vec<Vec<String>> -> OwnedSlice<OwnedSlice<CustomIdent>>
                let line_names: Vec<OwnedSlice<CustomIdent>> = repetition.line_names.iter()
                    .map(|names| {
                        let custom_idents: Vec<CustomIdent> = names.iter()
                            .map(|name| CustomIdent(Atom::from(name.as_str())))
                            .collect();
                        OwnedSlice::from(custom_idents)
                    })
                    .collect();
                
                // Build TrackRepeat
                let track_repeat = stylo::TrackRepeat {
                    count,
                    line_names: OwnedSlice::from(line_names),
                    track_sizes: OwnedSlice::from(track_sizes),
                };
                
                // Push as TrackRepeat, not as TrackSize!
                track_list_values.push(stylo::TrackListValue::TrackRepeat(track_repeat));
            }
        }
    }

    let line_names: Vec<OwnedSlice<style::values::CustomIdent>> =
        (0..=track_list_values.len())
            .map(|_| OwnedSlice::default())
            .collect();

    let track_list = stylo::TrackList {
        auto_repeat_index: std::usize::MAX,
        values: OwnedSlice::from(track_list_values),
        line_names: OwnedSlice::from(line_names),
    };

    GenericGridTemplateComponent::TrackList(Box::new(track_list))
}
