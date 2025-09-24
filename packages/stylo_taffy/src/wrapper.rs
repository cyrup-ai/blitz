use std::ops::Deref;
use std::sync::OnceLock;

use style::properties::ComputedValues;
use taffy::prelude::FromLength;

use crate::convert;

/// Helper function to convert Vec<T> to slice iterator for function pointer compatibility
fn vec_iter<T>(vec: &Vec<T>) -> core::slice::Iter<'_, T> {
    vec.iter()
}

/// A wrapper struct for anything that `Deref`s to a [`stylo::ComputedValues`](ComputedValues) that caches
/// computed grid data to enable proper iterator borrowing.
pub struct TaffyStyloStyle<T: Deref<Target = ComputedValues>> {
    computed_values: T,
    grid_template_rows: OnceLock<Vec<taffy::GridTemplateComponent<String>>>,
    grid_template_columns: OnceLock<Vec<taffy::GridTemplateComponent<String>>>,
    grid_auto_rows: OnceLock<Vec<taffy::TrackSizingFunction>>,
    grid_auto_columns: OnceLock<Vec<taffy::TrackSizingFunction>>,
    grid_template_areas: OnceLock<Option<Vec<taffy::GridTemplateArea<String>>>>,
    grid_template_column_names: OnceLock<Option<Vec<Vec<String>>>>,
    grid_template_row_names: OnceLock<Option<Vec<Vec<String>>>>,
}

// Deref<stylo::ComputedValues> impl
impl<T: Deref<Target = ComputedValues>> From<T> for TaffyStyloStyle<T> {
    fn from(value: T) -> Self {
        Self {
            computed_values: value,
            grid_template_rows: OnceLock::new(),
            grid_template_columns: OnceLock::new(),
            grid_auto_rows: OnceLock::new(),
            grid_auto_columns: OnceLock::new(),
            grid_template_areas: OnceLock::new(),
            grid_template_column_names: OnceLock::new(),
            grid_template_row_names: OnceLock::new(),
        }
    }
}

// Into<taffy::Style> impl
impl<T: Deref<Target = ComputedValues>> From<TaffyStyloStyle<T>> for taffy::Style {
    fn from(value: TaffyStyloStyle<T>) -> Self {
        convert::to_taffy_style(&value.computed_values)
    }
}

// CoreStyle impl
impl<T: Deref<Target = ComputedValues>> taffy::CoreStyle for TaffyStyloStyle<T> {
    type CustomIdent = String;

    #[inline]
    fn box_generation_mode(&self) -> taffy::BoxGenerationMode {
        convert::box_generation_mode(self.computed_values.get_box().display)
    }

    #[inline]
    fn is_block(&self) -> bool {
        convert::is_block(self.computed_values.get_box().display)
    }

    #[inline]
    fn box_sizing(&self) -> taffy::BoxSizing {
        convert::box_sizing(self.computed_values.get_position().box_sizing)
    }

    #[inline]
    fn overflow(&self) -> taffy::Point<taffy::Overflow> {
        let box_styles = self.computed_values.get_box();
        taffy::Point {
            x: convert::overflow(box_styles.overflow_x),
            y: convert::overflow(box_styles.overflow_y),
        }
    }

    #[inline]
    fn scrollbar_width(&self) -> f32 {
        // Production fallback: Use standard 15px scrollbar width when Device context unavailable
        // For accurate platform-specific scrollbar width detection, use to_taffy_style_with_device()
        // which provides access to device.scrollbar_inline_size().px()
        15.0
    }

    #[inline]
    fn position(&self) -> taffy::Position {
        convert::position(self.computed_values.get_box().position)
    }

    #[inline]
    fn inset(&self) -> taffy::Rect<taffy::LengthPercentageAuto> {
        let position_styles = self.computed_values.get_position();
        taffy::Rect {
            left: convert::inset(&position_styles.left),
            right: convert::inset(&position_styles.right),
            top: convert::inset(&position_styles.top),
            bottom: convert::inset(&position_styles.bottom),
        }
    }

    #[inline]
    fn size(&self) -> taffy::Size<taffy::Dimension> {
        let position_styles = self.computed_values.get_position();
        taffy::Size {
            width: convert::dimension(&position_styles.width),
            height: convert::dimension(&position_styles.height),
        }
    }

    #[inline]
    fn min_size(&self) -> taffy::Size<taffy::Dimension> {
        let position_styles = self.computed_values.get_position();
        taffy::Size {
            width: convert::dimension(&position_styles.min_width),
            height: convert::dimension(&position_styles.min_height),
        }
    }

    #[inline]
    fn max_size(&self) -> taffy::Size<taffy::Dimension> {
        let position_styles = self.computed_values.get_position();
        taffy::Size {
            width: convert::max_size_dimension(&position_styles.max_width),
            height: convert::max_size_dimension(&position_styles.max_height),
        }
    }

    #[inline]
    fn aspect_ratio(&self) -> Option<f32> {
        convert::aspect_ratio(self.computed_values.get_position().aspect_ratio)
    }

    #[inline]
    fn margin(&self) -> taffy::Rect<taffy::LengthPercentageAuto> {
        let margin_styles = self.computed_values.get_margin();
        taffy::Rect {
            left: convert::margin(&margin_styles.margin_left),
            right: convert::margin(&margin_styles.margin_right),
            top: convert::margin(&margin_styles.margin_top),
            bottom: convert::margin(&margin_styles.margin_bottom),
        }
    }

    #[inline]
    fn padding(&self) -> taffy::Rect<taffy::LengthPercentage> {
        let padding_styles = self.computed_values.get_padding();
        taffy::Rect {
            left: convert::length_percentage(&padding_styles.padding_left.0),
            right: convert::length_percentage(&padding_styles.padding_right.0),
            top: convert::length_percentage(&padding_styles.padding_top.0),
            bottom: convert::length_percentage(&padding_styles.padding_bottom.0),
        }
    }

    #[inline]
    fn border(&self) -> taffy::Rect<taffy::LengthPercentage> {
        let border_styles = self.computed_values.get_border();
        taffy::Rect {
            left: taffy::LengthPercentage::from_length(border_styles.border_left_width.to_f32_px()),
            right: taffy::LengthPercentage::from_length(
                border_styles.border_right_width.to_f32_px(),
            ),
            top: taffy::LengthPercentage::from_length(border_styles.border_top_width.to_f32_px()),
            bottom: taffy::LengthPercentage::from_length(
                border_styles.border_bottom_width.to_f32_px(),
            ),
        }
    }
}

// BlockContainerStyle impl
#[cfg(feature = "block")]
impl<T: Deref<Target = ComputedValues>> taffy::BlockContainerStyle for TaffyStyloStyle<T> {
    #[inline]
    fn text_align(&self) -> taffy::TextAlign {
        convert::text_align(self.computed_values.clone_text_align())
    }
}

// BlockItemStyle impl
#[cfg(feature = "block")]
impl<T: Deref<Target = ComputedValues>> taffy::BlockItemStyle for TaffyStyloStyle<T> {
    #[inline]
    fn is_table(&self) -> bool {
        convert::is_table(self.computed_values.clone_display())
    }
}

// FlexboxContainerStyle impl
#[cfg(feature = "flexbox")]
impl<T: Deref<Target = ComputedValues>> taffy::FlexboxContainerStyle for TaffyStyloStyle<T> {
    #[inline]
    fn flex_direction(&self) -> taffy::FlexDirection {
        convert::flex_direction(self.computed_values.get_position().flex_direction)
    }

    #[inline]
    fn flex_wrap(&self) -> taffy::FlexWrap {
        convert::flex_wrap(self.computed_values.get_position().flex_wrap)
    }

    #[inline]
    fn gap(&self) -> taffy::Size<taffy::LengthPercentage> {
        let position_styles = self.computed_values.get_position();
        taffy::Size {
            width: convert::gap(&position_styles.column_gap),
            height: convert::gap(&position_styles.row_gap),
        }
    }

    #[inline]
    fn align_content(&self) -> Option<taffy::AlignContent> {
        convert::content_alignment(self.computed_values.get_position().align_content.0)
    }

    #[inline]
    fn align_items(&self) -> Option<taffy::AlignItems> {
        convert::item_alignment(self.computed_values.get_position().align_items.0)
    }

    #[inline]
    fn justify_content(&self) -> Option<taffy::JustifyContent> {
        convert::content_alignment(self.computed_values.get_position().justify_content.0)
    }
}

// FlexboxItemStyle impl
#[cfg(feature = "flexbox")]
impl<T: Deref<Target = ComputedValues>> taffy::FlexboxItemStyle for TaffyStyloStyle<T> {
    #[inline]
    fn flex_basis(&self) -> taffy::Dimension {
        convert::flex_basis(&self.computed_values.get_position().flex_basis)
    }

    #[inline]
    fn flex_grow(&self) -> f32 {
        self.computed_values.get_position().flex_grow.0
    }

    #[inline]
    fn flex_shrink(&self) -> f32 {
        self.computed_values.get_position().flex_shrink.0
    }

    #[inline]
    fn align_self(&self) -> Option<taffy::AlignSelf> {
        convert::item_alignment(self.computed_values.get_position().align_self.0.0)
    }
}

// GridContainerStyle impl
#[cfg(feature = "grid")]
impl<T: Deref<Target = ComputedValues>> taffy::GridContainerStyle for TaffyStyloStyle<T> {
    type Repetition<'a>
        = &'a taffy::GridTemplateRepetition<String>
    where
        Self: 'a;

    type TemplateTrackList<'a>
        = core::iter::Map<
        core::slice::Iter<'a, taffy::GridTemplateComponent<String>>,
        fn(
            &'a taffy::GridTemplateComponent<String>,
        ) -> taffy::GenericGridTemplateComponent<
            String,
            &'a taffy::GridTemplateRepetition<String>,
        >,
    >
    where
        Self: 'a;

    type AutoTrackList<'a>
        = core::iter::Copied<core::slice::Iter<'a, taffy::TrackSizingFunction>>
    where
        Self: 'a;

    type TemplateLineNames<'a>
        = core::iter::Map<
        core::slice::Iter<'a, Vec<String>>,
        fn(&Vec<String>) -> core::slice::Iter<'_, String>,
    >
    where
        Self: 'a;

    type GridTemplateAreas<'a>
        = core::iter::Cloned<core::slice::Iter<'a, taffy::GridTemplateArea<String>>>
    where
        Self: 'a;

    #[inline]
    fn grid_template_rows(&self) -> Option<Self::TemplateTrackList<'_>> {
        let tracks = self.grid_template_rows.get_or_init(|| {
            let pos = self.computed_values.get_position();
            crate::convert::grid_template_tracks(&pos.grid_template_rows, None)
        });
        if tracks.is_empty() {
            None
        } else {
            Some(
                tracks
                    .iter()
                    .map(taffy::GridTemplateComponent::as_component_ref),
            )
        }
    }

    #[inline]
    fn grid_template_columns(&self) -> Option<Self::TemplateTrackList<'_>> {
        let tracks = self.grid_template_columns.get_or_init(|| {
            let pos = self.computed_values.get_position();
            crate::convert::grid_template_tracks(&pos.grid_template_columns, None)
        });
        if tracks.is_empty() {
            None
        } else {
            Some(
                tracks
                    .iter()
                    .map(taffy::GridTemplateComponent::as_component_ref),
            )
        }
    }

    #[inline]
    fn grid_auto_rows(&self) -> Self::AutoTrackList<'_> {
        let tracks = self.grid_auto_rows.get_or_init(|| {
            let pos = self.computed_values.get_position();
            crate::convert::grid_auto_tracks(&pos.grid_auto_rows)
        });
        tracks.iter().copied()
    }

    #[inline]
    fn grid_auto_columns(&self) -> Self::AutoTrackList<'_> {
        let tracks = self.grid_auto_columns.get_or_init(|| {
            let pos = self.computed_values.get_position();
            crate::convert::grid_auto_tracks(&pos.grid_auto_columns)
        });
        tracks.iter().copied()
    }

    #[inline]
    fn grid_auto_flow(&self) -> taffy::GridAutoFlow {
        convert::grid_auto_flow(self.computed_values.get_position().grid_auto_flow)
    }

    #[inline]
    fn gap(&self) -> taffy::Size<taffy::LengthPercentage> {
        let position_styles = self.computed_values.get_position();
        taffy::Size {
            width: convert::gap(&position_styles.column_gap),
            height: convert::gap(&position_styles.row_gap),
        }
    }

    #[inline]
    fn align_content(&self) -> Option<taffy::AlignContent> {
        convert::content_alignment(self.computed_values.get_position().align_content.0)
    }

    #[inline]
    fn justify_content(&self) -> Option<taffy::JustifyContent> {
        convert::content_alignment(self.computed_values.get_position().justify_content.0)
    }

    #[inline]
    fn align_items(&self) -> Option<taffy::AlignItems> {
        convert::item_alignment(self.computed_values.get_position().align_items.0)
    }

    #[inline]
    fn justify_items(&self) -> Option<taffy::AlignItems> {
        convert::item_alignment(self.computed_values.get_position().justify_items.computed.0)
    }

    fn grid_template_areas(&self) -> Option<Self::GridTemplateAreas<'_>> {
        let areas = self.grid_template_areas.get_or_init(|| {
            let pos = self.computed_values.get_position();
            crate::convert::grid_template_areas(&pos.grid_template_areas)
        });
        areas.as_ref().map(|a| a.iter().cloned())
    }

    fn grid_template_column_names(&self) -> Option<Self::TemplateLineNames<'_>> {
        let names = self.grid_template_column_names.get_or_init(|| {
            let pos = self.computed_values.get_position();
            crate::convert::grid_template_line_names(&pos.grid_template_columns)
        });
        names.as_ref().map(|names| {
            names
                .iter()
                .map(vec_iter as fn(&Vec<String>) -> core::slice::Iter<'_, String>)
        })
    }

    fn grid_template_row_names(&self) -> Option<Self::TemplateLineNames<'_>> {
        let names = self.grid_template_row_names.get_or_init(|| {
            let pos = self.computed_values.get_position();
            crate::convert::grid_template_line_names(&pos.grid_template_rows)
        });
        names.as_ref().map(|names| {
            names
                .iter()
                .map(vec_iter as fn(&Vec<String>) -> core::slice::Iter<'_, String>)
        })
    }
}

// GridItemStyle impl
#[cfg(feature = "grid")]
impl<T: Deref<Target = ComputedValues>> taffy::GridItemStyle for TaffyStyloStyle<T> {
    #[inline]
    fn grid_row(&self) -> taffy::Line<taffy::GridPlacement> {
        let position_styles = self.computed_values.get_position();
        taffy::Line {
            start: convert::grid_line(&position_styles.grid_row_start),
            end: convert::grid_line(&position_styles.grid_row_end),
        }
    }

    #[inline]
    fn grid_column(&self) -> taffy::Line<taffy::GridPlacement> {
        let position_styles = self.computed_values.get_position();
        taffy::Line {
            start: convert::grid_line(&position_styles.grid_column_start),
            end: convert::grid_line(&position_styles.grid_column_end),
        }
    }

    #[inline]
    fn align_self(&self) -> Option<taffy::AlignSelf> {
        convert::item_alignment(self.computed_values.get_position().align_self.0.0)
    }

    #[inline]
    fn justify_self(&self) -> Option<taffy::AlignSelf> {
        convert::item_alignment(self.computed_values.get_position().justify_self.0.0)
    }
}

// TaffyStyloStyle constructor methods are in the main impl block above

// Subgrid and masonry detection methods for TaffyStyloStyle
impl<T: Deref<Target = ComputedValues>> TaffyStyloStyle<T> {
    /// Create a mutable wrapper for style modification (enables subgrid track inheritance)
    #[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid - used when full subgrid implementation is activated
    pub fn from_mut(computed_values: &mut ComputedValues) -> TaffyStyloStyleMut<'_> {
        TaffyStyloStyleMut::new(computed_values)
    }
}

impl<T: Deref<Target = ComputedValues>> TaffyStyloStyle<T> {
    /// Detects if this grid container uses subgrid for rows
    #[inline]
    pub fn has_subgrid_rows(&self) -> bool {
        use crate::convert::stylo::GenericGridTemplateComponent;
        let pos = self.computed_values.get_position();
        matches!(
            pos.grid_template_rows,
            GenericGridTemplateComponent::Subgrid(_)
        )
    }

    /// Detects if this grid container uses subgrid for columns  
    #[inline]
    pub fn has_subgrid_columns(&self) -> bool {
        use crate::convert::stylo::GenericGridTemplateComponent;
        let pos = self.computed_values.get_position();
        matches!(
            pos.grid_template_columns,
            GenericGridTemplateComponent::Subgrid(_)
        )
    }

    /// Detects if this grid container uses masonry for rows
    #[inline]
    pub fn has_masonry_rows(&self) -> bool {
        use crate::convert::stylo::GenericGridTemplateComponent;
        let pos = self.computed_values.get_position();
        matches!(
            pos.grid_template_rows,
            GenericGridTemplateComponent::Masonry
        )
    }

    /// Detects if this grid container uses masonry for columns
    #[inline]
    pub fn has_masonry_columns(&self) -> bool {
        use crate::convert::stylo::GenericGridTemplateComponent;
        let pos = self.computed_values.get_position();
        matches!(
            pos.grid_template_columns,
            GenericGridTemplateComponent::Masonry
        )
    }

    /// Gets the raw stylo grid template rows for preprocessing
    #[inline]
    pub fn raw_grid_template_rows(&self) -> &crate::convert::stylo::GridTemplateComponent {
        &self.computed_values.get_position().grid_template_rows
    }

    /// Gets the raw stylo grid template columns for preprocessing  
    #[inline]
    pub fn raw_grid_template_columns(&self) -> &crate::convert::stylo::GridTemplateComponent {
        &self.computed_values.get_position().grid_template_columns
    }
}

/// A mutable wrapper that enables CSS style modification for subgrid track inheritance
///
/// This wrapper allows modification of cached grid template values to implement subgrid
/// track inheritance without violating stylo's immutable computed values architecture.
#[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid - used when full subgrid implementation is activated
pub struct TaffyStyloStyleMut<'a> {
    /// The underlying TaffyStyloStyle with cached values that can be modified
    style: TaffyStyloStyle<&'a mut ComputedValues>,
}

impl<'a> TaffyStyloStyleMut<'a> {
    /// Create a new mutable wrapper around ComputedValues
    #[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid - used when full subgrid implementation is activated
    pub fn new(computed_values: &'a mut ComputedValues) -> Self {
        Self {
            style: TaffyStyloStyle::from(computed_values),
        }
    }

    /// Set grid template rows for subgrid track inheritance
    ///
    /// This works by overriding the cached grid template rows with inherited parent tracks,
    /// allowing subgrid behavior without modifying immutable computed values.
    #[cfg(feature = "grid")]
    #[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid - used when full subgrid implementation is activated
    pub fn set_grid_template_rows(
        &mut self,
        tracks: Vec<taffy::TrackSizingFunction>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert TrackSizingFunction to GridTemplateComponent for caching
        let grid_components: Vec<taffy::GridTemplateComponent<String>> = tracks
            .into_iter()
            .map(|track| taffy::GridTemplateComponent::Single(track))
            .collect();

        // Override the cached grid template rows with inherited tracks
        // This bypasses the computed values and directly sets what the grid layout algorithm will see
        let _ = self.style.grid_template_rows.set(grid_components);

        Ok(())
    }

    /// Set grid template columns for subgrid track inheritance
    ///
    /// This works by overriding the cached grid template columns with inherited parent tracks,
    /// allowing subgrid behavior without modifying immutable computed values.
    #[cfg(feature = "grid")]
    #[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid - used when full subgrid implementation is activated
    pub fn set_grid_template_columns(
        &mut self,
        tracks: Vec<taffy::TrackSizingFunction>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert TrackSizingFunction to GridTemplateComponent for caching
        let grid_components: Vec<taffy::GridTemplateComponent<String>> = tracks
            .into_iter()
            .map(|track| taffy::GridTemplateComponent::Single(track))
            .collect();

        // Override the cached grid template columns with inherited tracks
        // This bypasses the computed values and directly sets what the grid layout algorithm will see
        let _ = self.style.grid_template_columns.set(grid_components);

        Ok(())
    }

    /// Set grid template row names for subgrid track inheritance
    ///
    /// This works by overriding the cached line names with inherited parent line names.
    #[cfg(feature = "grid")]
    #[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid - used when full subgrid implementation is activated
    pub fn set_grid_template_row_names(
        &mut self,
        names: Vec<Vec<String>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Override the cached grid template row names with inherited names
        let _ = self.style.grid_template_row_names.set(Some(names));
        Ok(())
    }

    /// Set grid template column names for subgrid track inheritance
    ///
    /// This works by overriding the cached line names with inherited parent line names.
    #[cfg(feature = "grid")]
    #[allow(dead_code)] // Infrastructure for CSS Grid Level 2 subgrid - used when full subgrid implementation is activated
    pub fn set_grid_template_column_names(
        &mut self,
        names: Vec<Vec<String>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Override the cached grid template column names with inherited names
        let _ = self.style.grid_template_column_names.set(Some(names));
        Ok(())
    }
}

// Mutable accessor methods for subgrid integration
impl<T: std::ops::DerefMut<Target = ComputedValues>> TaffyStyloStyle<T> {
    /// Get mutable access to the underlying ComputedValues for subgrid track inheritance
    #[cfg(feature = "grid")]
    pub fn computed_values_mut(&mut self) -> &mut ComputedValues {
        &mut self.computed_values
    }
}
