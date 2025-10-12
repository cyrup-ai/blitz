//! CSS Grid alignment for masonry items
//!
//! Based on Taffy's grid alignment implementation.

use taffy::prelude::NodeId;
use taffy::geometry::AbstractAxis;
use taffy::{AlignSelf, FlexboxItemStyle, GridItemStyle};

use crate::BaseDocument;

/// Get alignment values for a masonry item
/// Returns (grid_axis_alignment, masonry_axis_alignment)
pub fn get_masonry_item_alignment(
    tree: &BaseDocument,
    item_id: NodeId,
    masonry_axis: AbstractAxis,
) -> (Option<AlignSelf>, Option<AlignSelf>) {
    let node = tree.node_from_id(item_id.into());
    
    if let Some(styles) = node.primary_styles() {
        let wrapper = stylo_taffy::TaffyStyloStyle::from(styles);
        
        match masonry_axis {
            AbstractAxis::Block => {
                // Masonry flows vertically (block), grid axis is horizontal (inline)
                let justify = GridItemStyle::justify_self(&wrapper);  // Grid axis
                let align = FlexboxItemStyle::align_self(&wrapper);   // Masonry axis
                (justify, align)
            }
            AbstractAxis::Inline => {
                // Masonry flows horizontally (inline), grid axis is vertical (block)
                let justify = FlexboxItemStyle::align_self(&wrapper);   // Masonry axis
                let align = GridItemStyle::justify_self(&wrapper);      // Grid axis
                (align, justify)
            }
        }
    } else {
        (None, None)
    }
}

/// Align an item within its grid area along a single axis
/// Based on Taffy's align_item_within_area() function
///
/// Returns: aligned_position
pub fn align_item_within_area(
    grid_area_start: f32,
    grid_area_end: f32,
    alignment_style: AlignSelf,
    resolved_size: f32,
    baseline_shim: f32,
) -> f32 {
    let grid_area_size = (grid_area_end - grid_area_start).max(0.0);

    let alignment_offset = match alignment_style {
        AlignSelf::Start | AlignSelf::FlexStart => baseline_shim,
        AlignSelf::End | AlignSelf::FlexEnd => grid_area_size - resolved_size,
        AlignSelf::Center => (grid_area_size - resolved_size) / 2.0,
        AlignSelf::Baseline => baseline_shim,  // Shim already calculated
        AlignSelf::Stretch => 0.0,  // Item already stretched during sizing
    };

    grid_area_start + alignment_offset
}

/// Check if item should stretch in the given axis
pub fn should_stretch(alignment: Option<AlignSelf>) -> bool {
    matches!(
        alignment,
        None | Some(AlignSelf::Stretch)
    )
}
