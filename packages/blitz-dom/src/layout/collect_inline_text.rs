// Helper function to collect text content from inline nodes
// Replaces the complex parley tree building with simple text collection

use slab::Slab;
use style::values::computed::Display;
use style::values::specified::box_::{DisplayInside, DisplayOutside};

use crate::layout::stylo_to_blitz::TextCollapseMode;
use crate::node::{Node, NodeData, SpecialElementData};

/// Collect text content from inline nodes recursively
/// Simplified version of build_inline_layout_recursive that just extracts text
pub(crate) fn collect_inline_text_recursive(
    text_content: &mut String,
    nodes: &Slab<Node>,
    node_id: usize,
    collapse_mode: TextCollapseMode,
) {
    let node = &nodes[node_id];

    match &node.data {
        NodeData::Element(element_data) | NodeData::AnonymousBlock(element_data) => {
            // Skip hidden input elements
            if *element_data.name.local == *"input"
                && element_data.attr(markup5ever::local_name!("type")) == Some("hidden")
            {
                return;
            }

            let display = node.display_style().unwrap_or(Display::inline());

            match (display.outside(), display.inside()) {
                (DisplayOutside::None, DisplayInside::None) => {}
                (DisplayOutside::None, DisplayInside::Contents) => {
                    // Recurse into display:contents nodes
                    for child_id in node.children.iter().copied() {
                        collect_inline_text_recursive(text_content, nodes, child_id, collapse_mode);
                    }
                }
                (DisplayOutside::Inline, DisplayInside::Flow) => {
                    let tag_name = &element_data.name.local;

                    // Handle special elements
                    if *tag_name == markup5ever::local_name!("br") {
                        text_content.push('\n');
                    } else if is_replaced_element(&element_data.special_data, tag_name) {
                        // Replaced elements don't contribute text content
                        // but they take up space in layout
                    } else {
                        // Recurse into children for text content
                        if let Some(before_id) = node.before {
                            collect_inline_text_recursive(
                                text_content,
                                nodes,
                                before_id,
                                collapse_mode,
                            );
                        }
                        for child_id in node.children.iter().copied() {
                            collect_inline_text_recursive(
                                text_content,
                                nodes,
                                child_id,
                                collapse_mode,
                            );
                        }
                        if let Some(after_id) = node.after {
                            collect_inline_text_recursive(
                                text_content,
                                nodes,
                                after_id,
                                collapse_mode,
                            );
                        }
                    }
                }
                // Inline box - doesn't contribute text but may have children
                (_, _) => {}
            }
        }
        NodeData::Text(text_data) => {
            // Apply whitespace collapsing based on mode
            let processed_text = match collapse_mode {
                TextCollapseMode::Collapse => {
                    // Normal HTML whitespace collapsing
                    let collapsed = text_data
                        .content
                        .chars()
                        .fold((String::new(), false), |(mut acc, prev_was_space), c| {
                            if c.is_ascii_whitespace() {
                                if !prev_was_space {
                                    acc.push(' ');
                                }
                                (acc, true)
                            } else {
                                acc.push(c);
                                (acc, false)
                            }
                        })
                        .0;
                    collapsed
                }
                TextCollapseMode::Preserve => {
                    // Preserve all whitespace (like pre)
                    text_data.content.clone()
                }
                TextCollapseMode::PreserveNewlines => {
                    // Preserve newlines but collapse other spaces
                    text_data
                        .content
                        .lines()
                        .map(|line| {
                            line.chars()
                                .fold((String::new(), false), |(mut acc, prev_was_space), c| {
                                    if c.is_ascii_whitespace() && c != '\n' {
                                        if !prev_was_space {
                                            acc.push(' ');
                                        }
                                        (acc, true)
                                    } else {
                                        acc.push(c);
                                        (acc, false)
                                    }
                                })
                                .0
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                TextCollapseMode::PreserveBreakable => {
                    // Like preserve but allows line breaking
                    text_data.content.clone()
                }
            };

            text_content.push_str(&processed_text);
        }
        NodeData::Comment => {
            // Comments don't contribute to text content
        }
        NodeData::Document => {
            // Document node shouldn't appear in inline context
            unreachable!("Document node in inline context")
        }
    }
}

/// Check if an element is a replaced element
#[inline]
fn is_replaced_element(
    special_data: &SpecialElementData,
    tag_name: &markup5ever::LocalName,
) -> bool {
    matches!(
        special_data,
        SpecialElementData::Image(_)
            | SpecialElementData::Canvas(_)
            | SpecialElementData::TextInput(_)
            | SpecialElementData::CheckboxInput(_)
    ) || matches!(
        *tag_name,
        markup5ever::local_name!("canvas")
            | markup5ever::local_name!("img")
            | markup5ever::local_name!("svg")
            | markup5ever::local_name!("textarea")
            | markup5ever::local_name!("button")
    )
}
