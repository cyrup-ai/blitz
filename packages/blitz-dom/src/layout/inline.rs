use blitz_text::Align as CosmicAlign;
use taffy::{
    AvailableSpace, LayoutPartialTree as _, MaybeMath as _, MaybeResolve as _, NodeId, Position,
    ResolveOrZero as _, Size, compute_leaf_layout,
};

use super::resolve_calc_value;
use crate::BaseDocument;

impl BaseDocument {
    pub(crate) fn compute_inline_layout(
        &mut self,
        node_id: usize,
        inputs: taffy::tree::LayoutInput,
    ) -> taffy::LayoutOutput {
        let scale = self.viewport.scale();

        // Take inline layout to satisfy borrow checker
        let element = match self.nodes[node_id].data.downcast_element_mut() {
            Some(element) => element,
            None => {
                eprintln!(
                    "Warning: Cannot measure inline layout for node {}: node is not an element",
                    node_id
                );
                return taffy::LayoutOutput {
                    size: taffy::Size::ZERO,
                    content_size: taffy::Size::ZERO,
                    first_baselines: taffy::Point::NONE,
                    top_margin: taffy::CollapsibleMarginSet::ZERO,
                    bottom_margin: taffy::CollapsibleMarginSet::ZERO,
                    margins_can_collapse_through: false,
                };
            }
        };

        let mut inline_layout = match element.take_inline_layout() {
            Some(layout) => layout,
            None => {
                eprintln!(
                    "Warning: Cannot measure inline layout for node {}: element has no inline layout data",
                    node_id
                );
                return taffy::LayoutOutput {
                    size: taffy::Size::ZERO,
                    content_size: taffy::Size::ZERO,
                    first_baselines: taffy::Point::NONE,
                    top_margin: taffy::CollapsibleMarginSet::ZERO,
                    bottom_margin: taffy::CollapsibleMarginSet::ZERO,
                    margins_can_collapse_through: false,
                };
            }
        };

        // TODO: eliminate clone
        let style = self.nodes[node_id].style().clone();

        let output = compute_leaf_layout(
            inputs,
            &style,
            resolve_calc_value,
            |_known_dimensions, available_space| {
                // Short circuit if inline context contains no text or inline boxes
                if inline_layout.text.is_empty() && inline_layout.inline_boxes.is_empty() {
                    return Size::ZERO;
                }

                // Compute size of inline boxes
                let child_inputs = taffy::tree::LayoutInput {
                    known_dimensions: Size::NONE,
                    available_space,
                    parent_size: available_space.into_options(),
                    ..inputs
                };
                for ibox in &mut inline_layout.inline_boxes {
                    let style = &self.nodes[ibox.id as usize].style();
                    let margin = style
                        .margin
                        .resolve_or_zero(inputs.parent_size, resolve_calc_value);

                    if style.position == Position::Absolute {
                        ibox.width = 0.0;
                        ibox.height = 0.0;
                    } else {
                        let output = self.compute_child_layout(NodeId::from(ibox.id), child_inputs);
                        ibox.width = (margin.left + margin.right + output.size.width) * scale;
                        ibox.height = (margin.top + margin.bottom + output.size.height) * scale;
                    }
                }

                // Determine width
                let padding = style
                    .padding
                    .resolve_or_zero(inputs.parent_size, resolve_calc_value);
                let border = style
                    .border
                    .resolve_or_zero(inputs.parent_size, resolve_calc_value);
                let container_pb = padding + border;
                let pbw = container_pb.horizontal_components().sum() * scale;

                let width = inputs
                    .known_dimensions
                    .width
                    .map(|w| (w * scale) - pbw)
                    .unwrap_or_else(|| {
                        // Get font system for content width calculation
                        let content_sizes = self.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                            inline_layout.calculate_content_widths_with_inline_elements(font_system)
                        })).unwrap_or_else(|_| {
                            // If text system is not available, provide reasonable defaults
                            crate::node::ContentWidths { min: 0.0, max: 0.0 }
                        });
                        let computed_width = match available_space.width {
                            AvailableSpace::MinContent => content_sizes.min,
                            AvailableSpace::MaxContent => content_sizes.max,
                            AvailableSpace::Definite(limit) => (limit * scale)
                                .min(content_sizes.max)
                                .max(content_sizes.min),
                        }
                        .ceil();
                        let style_width = style
                            .size
                            .width
                            .maybe_resolve(inputs.parent_size.width, resolve_calc_value)
                            .map(|w| w * scale);
                        let min_width = style
                            .min_size
                            .width
                            .maybe_resolve(inputs.parent_size.width, resolve_calc_value)
                            .map(|w| w * scale);
                        let max_width = style
                            .max_size
                            .width
                            .maybe_resolve(inputs.parent_size.width, resolve_calc_value)
                            .map(|w| w * scale);

                        (style_width)
                            .unwrap_or(computed_width + pbw)
                            .max(computed_width)
                            .maybe_clamp(min_width, max_width)
                            - pbw
                    });

                // Perform inline layout
                let _ = self.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                    inline_layout.break_all_lines(font_system, Some(width));
                }));

                if inputs.run_mode == taffy::RunMode::ComputeSize {
                    return taffy::Size {
                        width: width.ceil() / scale,
                        // Height will be ignored in RequestedAxis is Horizontal
                        height: inline_layout.height() / scale,
                    };
                }

                let _alignment = self.nodes[node_id]
                    .primary_styles()
                    .map(|s| {
                        use style::values::specified::TextAlignKeyword;

                        match s.clone_text_align() {
                            TextAlignKeyword::Start
                            | TextAlignKeyword::Left
                            | TextAlignKeyword::MozLeft => CosmicAlign::Left,
                            TextAlignKeyword::Right | TextAlignKeyword::MozRight => {
                                CosmicAlign::Right
                            }
                            TextAlignKeyword::Center | TextAlignKeyword::MozCenter => {
                                CosmicAlign::Center
                            }
                            TextAlignKeyword::Justify => CosmicAlign::Justified,
                            TextAlignKeyword::End => CosmicAlign::Right,
                        }
                    })
                    .unwrap_or(CosmicAlign::Left);

                // Set alignment in cosmyc-text buffer
                // Note: cosmyc-text handles alignment differently - it's set when creating the buffer
                // For existing buffer, we would need to recreate it with new alignment
                // For now, we'll track the alignment for future buffer operations

                // Store sizes and positions of inline boxes
                // Updated for cosmyc-text system - iterate through inline_layout.inline_boxes
                let container_pb = style
                    .padding
                    .resolve_or_zero(inputs.parent_size, resolve_calc_value)
                    + style
                        .border
                        .resolve_or_zero(inputs.parent_size, resolve_calc_value);

                for ibox in &inline_layout.inline_boxes {
                    let node = &self.nodes[ibox.id as usize];
                    let padding = node
                        .style()
                        .padding
                        .resolve_or_zero(child_inputs.parent_size, resolve_calc_value);
                    let border = node
                        .style()
                        .border
                        .resolve_or_zero(child_inputs.parent_size, resolve_calc_value);
                    let margin = node
                        .style()
                        .margin
                        .resolve_or_zero(child_inputs.parent_size, resolve_calc_value);

                    // Resolve inset values for absolute positioning
                    let left = node
                        .style()
                        .inset
                        .left
                        .maybe_resolve(child_inputs.parent_size.width, resolve_calc_value);
                    let right = node
                        .style()
                        .inset
                        .right
                        .maybe_resolve(child_inputs.parent_size.width, resolve_calc_value);
                    let top = node
                        .style()
                        .inset
                        .top
                        .maybe_resolve(child_inputs.parent_size.height, resolve_calc_value);
                    let bottom = node
                        .style()
                        .inset
                        .bottom
                        .maybe_resolve(child_inputs.parent_size.height, resolve_calc_value);

                    if node.style().position == Position::Absolute {
                        // Handle absolute positioning
                        let output = self.compute_child_layout(NodeId::from(ibox.id), child_inputs);

                        let layout = &mut self.nodes[ibox.id as usize].unrounded_layout;
                        layout.size = output.size;

                        // Calculate absolute position based on inset values or fallback to inline position
                        layout.location.x = left
                            .or_else(|| {
                                child_inputs.parent_size.width.zip(right).map(
                                    |(parent_width, right_offset)| {
                                        parent_width - right_offset - layout.size.width
                                    },
                                )
                            })
                            .unwrap_or((ibox.x / scale) + margin.left + container_pb.left);

                        layout.location.y = top
                            .or_else(|| {
                                child_inputs.parent_size.height.zip(bottom).map(
                                    |(parent_height, bottom_offset)| {
                                        parent_height - bottom_offset - layout.size.height
                                    },
                                )
                            })
                            .unwrap_or((ibox.y / scale) + margin.top + container_pb.top);

                        layout.padding = padding;
                        layout.border = border;
                    } else {
                        // Handle relative/static positioning - use inline box coordinates
                        let layout = &mut self.nodes[ibox.id as usize].unrounded_layout;
                        layout.size.width = (ibox.width / scale) - margin.left - margin.right;
                        layout.size.height = (ibox.height / scale) - margin.top - margin.bottom;
                        layout.location.x = (ibox.x / scale) + margin.left + container_pb.left;
                        layout.location.y = (ibox.y / scale) + margin.top + container_pb.top;
                        layout.padding = padding;
                        layout.border = border;
                    }
                }

                // println!("INLINE LAYOUT FOR {:?}. max_advance: {:?}", node_id, max_advance);
                // dbg!(&inline_layout.text);
                // println!("Computed: w: {} h: {}", inline_layout.layout.width(), inline_layout.layout.height());
                // println!("known_dimensions: w: {:?} h: {:?}", inputs.known_dimensions.width, inputs.known_dimensions.height);
                // println!("\n");

                taffy::Size {
                    width: inputs
                        .known_dimensions
                        .width
                        .unwrap_or_else(|| inline_layout.width().ceil() / scale),
                    height: inputs
                        .known_dimensions
                        .height
                        .unwrap_or_else(|| inline_layout.height() / scale),
                }
            },
        );

        // Put layout back
        match self.nodes[node_id].data.downcast_element_mut() {
            Some(element) => {
                element.inline_layout_data = Some(inline_layout);
            }
            None => {
                eprintln!(
                    "Warning: Cannot restore inline layout for node {}: element was removed during layout",
                    node_id
                );
            }
        }

        output
    }
}
