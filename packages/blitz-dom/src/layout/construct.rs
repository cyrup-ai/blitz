use core::str;
use std::sync::Arc;

// Replaced parley with cosmyc-text for text processing
use blitz_text::Edit;
use markup5ever::{QualName, local_name, ns};
use style::{
    data::ElementData as StyloElementData,
    properties::longhands::{
        list_style_position::computed_value::T as ListStylePosition,
        list_style_type::computed_value::T as ListStyleType,
    },
    shared_lock::StylesheetGuards,
    values::{
        computed::{Content, ContentItem, Display},
        specified::box_::{DisplayInside, DisplayOutside},
    },
};

use super::{
    collect_inline_text::collect_inline_text_recursive, stylo_to_blitz, table::build_table_context,
};
use crate::{
    BaseDocument, ElementData, Node, NodeData,
    node::{
        ListItemLayout, ListItemLayoutPosition, Marker, NodeFlags, NodeKind, SpecialElementData,
        TextInputData, TextLayout,
    },
};

const DUMMY_NAME: QualName = QualName {
    prefix: None,
    ns: ns!(html),
    local: local_name!("div"),
};

fn push_children_and_pseudos(layout_children: &mut Vec<usize>, node: &Node) {
    if let Some(before) = node.before {
        layout_children.push(before);
    }
    layout_children.extend_from_slice(&node.children);
    if let Some(after) = node.after {
        layout_children.push(after);
    }
}

/// Convert a relative line height to an absolute one - cosmyc-text version
fn resolve_line_height(line_height: f32, font_size: f32) -> f32 {
    // Cosmic-text line height is already absolute pixels
    // If it's relative (< 3.0), treat as multiplier, otherwise as absolute
    if line_height < 3.0 {
        line_height * font_size
    } else {
        line_height
    }
}

pub(crate) fn collect_layout_children(
    doc: &mut BaseDocument,
    container_node_id: usize,
    layout_children: &mut Vec<usize>,
    anonymous_block_id: &mut Option<usize>,
) {
    // Reset construction flags
    // TODO: make incremental and only remove this if the element is no longer an inline root
    doc.nodes[container_node_id]
        .flags
        .reset_construction_flags();
    if let Some(element_data) = doc.nodes[container_node_id].element_data_mut() {
        element_data.take_inline_layout();
    }

    flush_pseudo_elements(doc, container_node_id);

    if let Some(el) = doc.nodes[container_node_id].data.downcast_element() {
        // Handle text inputs
        let tag_name = el.name.local.as_ref();
        if matches!(tag_name, "input" | "textarea") {
            let type_attr: Option<&str> = doc.nodes[container_node_id]
                .data
                .downcast_element()
                .and_then(|el| el.attr(local_name!("type")));
            if tag_name == "textarea" {
                create_text_editor(doc, container_node_id, true);
                return;
            } else if matches!(
                type_attr,
                None | Some("text" | "password" | "email" | "number" | "search" | "tel" | "url")
            ) {
                create_text_editor(doc, container_node_id, false);
                return;
            } else if matches!(type_attr, Some("checkbox" | "radio")) {
                create_checkbox_input(doc, container_node_id);
                return;
            }
        }

        #[cfg(feature = "svg")]
        if matches!(tag_name, "svg") {
            let mut outer_html = match doc.get_node(container_node_id) {
                Some(node) => node.outer_html(),
                None => {
                    eprintln!(
                        "Warning: Cannot process SVG for node {}: node not found",
                        container_node_id
                    );
                    return;
                }
            };

            // HACK: usvg fails to parse SVGs that don't have the SVG xmlns set. So inject it
            // if the generated source doesn't have it.
            if !outer_html.contains("xmlns") {
                outer_html =
                    outer_html.replace("<svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"");
            }

            match crate::util::parse_svg(outer_html.as_bytes()) {
                Ok(svg) => {
                    let node = match doc.get_node_mut(container_node_id) {
                        Some(node) => node,
                        None => {
                            eprintln!(
                                "Warning: Cannot set SVG data for node {}: node not found",
                                container_node_id
                            );
                            return;
                        }
                    };
                    let element_data = match node.element_data_mut() {
                        Some(element) => element,
                        None => {
                            eprintln!(
                                "Warning: Cannot set SVG data for node {}: node is not an element",
                                container_node_id
                            );
                            return;
                        }
                    };
                    element_data.special_data = SpecialElementData::Image(Box::new(svg.into()));
                }
                Err(err) => {
                    println!("{container_node_id} SVG parse failed");
                    println!("{outer_html}");
                    dbg!(err);
                }
            };
            return;
        }

        // Only ol tags have start and reversed attributes
        let (mut index, reversed) = if tag_name == "ol" {
            (
                el.attr_parsed(local_name!("start"))
                    .map(|start: usize| start - 1)
                    .unwrap_or(0),
                el.attr_parsed(local_name!("reversed")).unwrap_or(false),
            )
        } else {
            (1, false)
        };
        collect_list_item_children(doc, &mut index, reversed, container_node_id);
    }

    // Skip further construction if the node has no children or psuedo-children
    // UNLESS it's a replaced element that needs to be rendered
    {
        let node = &doc.nodes[container_node_id];
        if node.children.is_empty() && node.before.is_none() && node.after.is_none() {
            // Check if this is a replaced element that should still be added to layout
            let is_replaced = node
                .element_data()
                .map(|el| {
                    // Check for replaced elements by special data or tag name
                    matches!(
                        el.special_data,
                        SpecialElementData::Image(_)
                            | SpecialElementData::Canvas(_)
                            | SpecialElementData::TextInput(_)
                            | SpecialElementData::CheckboxInput(_)
                    ) || matches!(
                        el.name.local.as_ref(),
                        "canvas" | "img" | "svg" | "input" | "textarea" | "button"
                    )
                })
                .unwrap_or(false);

            if !is_replaced {
                // Not a replaced element - skip it
                return;
            }
            // Replaced element - continue processing even without children
        }
    }

    let container_display = doc.nodes[container_node_id].display_style().unwrap_or(
        match doc.nodes[container_node_id].data.kind() {
            NodeKind::AnonymousBlock => Display::Block,
            _ => Display::Inline,
        },
    );

    match container_display.inside() {
        DisplayInside::None => {}
        DisplayInside::Contents => {
            // Take children array from node to avoid borrow checker issues.
            let children = std::mem::take(&mut doc.nodes[container_node_id].children);

            for child_id in children.iter().copied() {
                collect_layout_children(doc, child_id, layout_children, anonymous_block_id)
            }

            // Put children array back
            doc.nodes[container_node_id].children = children;
        }
        DisplayInside::Flow | DisplayInside::FlowRoot | DisplayInside::TableCell => {
            // TODO: make "all_inline" detection work in the presence of display:contents nodes
            let mut all_block = true;
            let mut all_inline = true;
            let mut has_contents = false;
            for child in doc.nodes[container_node_id]
                .children
                .iter()
                .copied()
                .map(|child_id| &doc.nodes[child_id])
            {
                // Unwraps on Text and SVG nodes
                let display = child.display_style().unwrap_or(Display::inline());
                if matches!(display.inside(), DisplayInside::Contents) {
                    has_contents = true;
                } else {
                    match display.outside() {
                        DisplayOutside::None => {}
                        DisplayOutside::Block
                        | DisplayOutside::TableCaption
                        | DisplayOutside::InternalTable => all_inline = false,
                        // Note: InternalRuby variant only available with gecko feature
                        DisplayOutside::Inline => {
                            all_block = false;

                            // We need the "complex" tree fixing when an inline contains a block
                            if child.is_or_contains_block() {
                                all_inline = false;
                            }
                        }
                    }
                }
            }

            // TODO: fix display:contents
            if all_inline {
                println!(
                    "🎯 INLINE LAYOUT PATH: all_inline=true for node {}",
                    container_node_id
                );
                let (inline_layout, ilayout_children) = build_inline_layout(doc, container_node_id);
                println!(
                    "🎯 build_inline_layout returned {} children: {:?}",
                    ilayout_children.len(),
                    ilayout_children
                );
                doc.nodes[container_node_id]
                    .flags
                    .insert(NodeFlags::IS_INLINE_ROOT);
                match doc.nodes[container_node_id].data.downcast_element_mut() {
                    Some(element) => {
                        element.inline_layout_data = Some(Box::new(inline_layout));
                    }
                    None => {
                        eprintln!(
                            "Warning: Cannot set inline layout data for node {}: node is not an element",
                            container_node_id
                        );
                        return;
                    }
                }
                if let Some(before) = doc.nodes[container_node_id].before {
                    layout_children.push(before);
                }
                layout_children.extend_from_slice(&ilayout_children);
                if let Some(after) = doc.nodes[container_node_id].after {
                    layout_children.push(after);
                }
                return;
            }

            // If the children are either all inline or all block then simply return the regular children
            // as the layout children
            if (all_block | all_inline) & !has_contents {
                return push_children_and_pseudos(layout_children, &doc.nodes[container_node_id]);
            }

            fn block_item_needs_wrap(
                child_node_kind: NodeKind,
                display_outside: DisplayOutside,
            ) -> bool {
                child_node_kind == NodeKind::Text || display_outside == DisplayOutside::Inline
            }
            collect_complex_layout_children(
                doc,
                container_node_id,
                layout_children,
                anonymous_block_id,
                false,
                block_item_needs_wrap,
            );
        }
        DisplayInside::Flex | DisplayInside::Grid => {
            let has_text_node_or_contents = doc.nodes[container_node_id]
                .children
                .iter()
                .copied()
                .map(|child_id| &doc.nodes[child_id])
                .any(|child| {
                    let display = child.display_style().unwrap_or(Display::inline());
                    let node_kind = child.data.kind();
                    display.inside() == DisplayInside::Contents || node_kind == NodeKind::Text
                });

            if !has_text_node_or_contents {
                return push_children_and_pseudos(layout_children, &doc.nodes[container_node_id]);
            }

            fn flex_or_grid_item_needs_wrap(
                child_node_kind: NodeKind,
                _display_outside: DisplayOutside,
            ) -> bool {
                child_node_kind == NodeKind::Text
            }
            collect_complex_layout_children(
                doc,
                container_node_id,
                layout_children,
                anonymous_block_id,
                true,
                flex_or_grid_item_needs_wrap,
            );
        }

        DisplayInside::Table => {
            let (table_context, tlayout_children) = build_table_context(doc, container_node_id);
            #[allow(clippy::arc_with_non_send_sync)]
            let data = SpecialElementData::TableRoot(Arc::new(table_context));
            doc.nodes[container_node_id]
                .flags
                .insert(NodeFlags::IS_TABLE_ROOT);
            match doc.nodes[container_node_id].data.downcast_element_mut() {
                Some(element) => {
                    element.special_data = data;
                }
                None => {
                    eprintln!(
                        "Warning: Cannot set table layout data for node {}: node is not an element",
                        container_node_id
                    );
                    return;
                }
            }
            if let Some(before) = doc.nodes[container_node_id].before {
                layout_children.push(before);
            }
            layout_children.extend_from_slice(&tlayout_children);
            if let Some(after) = doc.nodes[container_node_id].after {
                layout_children.push(after);
            }
        }

        _ => {
            push_children_and_pseudos(layout_children, &doc.nodes[container_node_id]);
        }
    }
}

fn flush_pseudo_elements(doc: &mut BaseDocument, node_id: usize) {
    let (before_style, after_style, before_node_id, after_node_id) = {
        let node = &doc.nodes[node_id];

        let before_node_id = node.before;
        let after_node_id = node.after;

        // Note: yes these are kinda backwards
        let style_data = node.stylo_element_data.borrow();
        let before_style = style_data
            .as_ref()
            .and_then(|d| d.styles.pseudos.as_array()[1].clone());
        let after_style = style_data
            .as_ref()
            .and_then(|d| d.styles.pseudos.as_array()[0].clone());

        (before_style, after_style, before_node_id, after_node_id)
    };

    // Sync pseudo element
    // TODO: Make incremental
    for (idx, pe_style, pe_node_id) in [
        (1, before_style, before_node_id),
        (0, after_style, after_node_id),
    ] {
        // Delete psuedo element if it exists but shouldn't
        if let (Some(pe_node_id), None) = (pe_node_id, &pe_style) {
            doc.remove_and_drop_pe(pe_node_id);
            doc.nodes[node_id].set_pe_by_index(idx, None);
        }

        // Create pseudo element if it should exist but doesn't
        if let (None, Some(pe_style)) = (pe_node_id, &pe_style) {
            let new_node_id = doc.create_node(NodeData::AnonymousBlock(ElementData::new(
                DUMMY_NAME,
                Vec::new(),
            )));
            doc.nodes[new_node_id].parent = Some(node_id);

            let content = &pe_style.as_ref().get_counters().content;
            if let Content::Items(item_data) = content {
                let items = &item_data.items[0..item_data.alt_start];
                match &items[0] {
                    ContentItem::String(owned_str) => {
                        let text_node_id = doc.create_text_node(owned_str);
                        doc.nodes[new_node_id].children.push(text_node_id);
                    }
                    _ => {
                        // TODO: other types of content
                    }
                }
            }

            let mut element_data = StyloElementData::default();
            element_data.styles.primary = Some(pe_style.clone());
            element_data.set_restyled();
            *doc.nodes[new_node_id].stylo_element_data.borrow_mut() = Some(element_data);

            doc.nodes[node_id].set_pe_by_index(idx, Some(new_node_id));
        }

        // Else: Update psuedo element
        if let (Some(pe_node_id), Some(pe_style)) = (pe_node_id, pe_style) {
            // TODO: Update content

            let mut node_styles = doc.nodes[pe_node_id].stylo_element_data.borrow_mut();
            let node_styles = match node_styles.as_mut() {
                Some(styles) => styles,
                None => {
                    eprintln!(
                        "Warning: Pseudo element node {} has no stylo element data",
                        pe_node_id
                    );
                    return;
                }
            };
            let primary_styles = &mut node_styles.styles.primary;

            match primary_styles.as_ref() {
                Some(current_style) => {
                    if !std::ptr::eq(&**current_style, &*pe_style) {
                        *primary_styles = Some(pe_style);
                        node_styles.set_restyled();
                    }
                }
                None => {
                    *primary_styles = Some(pe_style);
                    node_styles.set_restyled();
                }
            }
        }
    }
}

fn collect_list_item_children(
    doc: &mut BaseDocument,
    index: &mut usize,
    reversed: bool,
    node_id: usize,
) {
    let mut children = doc.nodes[node_id].children.clone();
    if reversed {
        children.reverse();
    }
    for child in children.into_iter() {
        if let Some(layout) = node_list_item_child(doc, child, *index) {
            let node = &mut doc.nodes[child];
            match node.element_data_mut() {
                Some(element_data) => {
                    element_data.list_item_data = Some(Box::new(layout));
                    *index += 1;
                    collect_list_item_children(doc, index, reversed, child);
                }
                None => {
                    eprintln!(
                        "Warning: Cannot set list item data for node {}: node is not an element",
                        child
                    );
                }
            }
        } else {
            // Unset marker in case it was previously set
            let node = &mut doc.nodes[child];
            if let Some(element_data) = node.element_data_mut() {
                element_data.list_item_data = None;
            }
        }
    }
}

// Return a child node which is of display: list-item
fn node_list_item_child(
    doc: &mut BaseDocument,
    child_id: usize,
    index: usize,
) -> Option<ListItemLayout> {
    let node = &doc.nodes[child_id];

    // We only care about elements with display: list-item (li's have this automatically)
    if !node
        .primary_styles()
        .is_some_and(|style| style.get_box().display.is_list_item())
    {
        return None;
    }

    // Break on container elements when already in a list
    if node
        .element_data()
        .map(|element_data| {
            matches!(
                element_data.name.local,
                local_name!("ol") | local_name!("ul"),
            )
        })
        .unwrap_or(false)
    {
        return None;
    };

    let styles = match node.primary_styles() {
        Some(styles) => styles,
        None => {
            eprintln!(
                "Warning: Node {} has no primary styles for list item processing",
                child_id
            );
            return None;
        }
    };
    let list_style_type = styles.clone_list_style_type();
    let list_style_position = styles.clone_list_style_position();
    let marker = marker_for_style(list_style_type, index)?;

    let position = match list_style_position {
        ListStylePosition::Inside => ListItemLayoutPosition::Inside,
        ListStylePosition::Outside => {
            let cosmyc_style = stylo_to_blitz::style(child_id, &styles);

            // Set appropriate font family for bullet symbols
            let attrs = if let Some(font_family) = stylo_to_blitz::font_for_bullet(list_style_type)
            {
                blitz_text::AttrsOwned {
                    family_owned: blitz_text::FamilyOwned::new(font_family),
                    ..cosmyc_style.attrs
                }
            } else {
                cosmyc_style.attrs
            };

            // Add the marker text to the buffer
            let text_content = match &marker {
                Marker::Char(char) => char.to_string(),
                Marker::String(str) => str.clone(),
            };

            let buffer = doc.text_system.with_font_system(|font_system| {
                let mut buffer = blitz_text::EnhancedBuffer::new(font_system, cosmyc_style.metrics);

                buffer.set_text_cached(
                    font_system,
                    &text_content,
                    &attrs.as_attrs(),
                    blitz_text::Shaping::Advanced,
                );
                buffer.set_wrap_cached(font_system, cosmyc_style.wrap);

                // Shape and lay out the text - calculate width from layout
                let mut layout_width = 0.0f32;
                for line in &buffer.inner().lines {
                    if let Some(layout_lines) = line.layout_opt() {
                        for layout_line in layout_lines {
                            layout_width = layout_width.max(layout_line.w);
                        }
                    }
                }

                buffer.set_size_cached(font_system, Some(layout_width), Some(f32::INFINITY));
                buffer
            });

            ListItemLayoutPosition::Outside(Box::new(buffer))
        }
    };

    Some(ListItemLayout { marker, position })
}

// Determine the marker to render for a given list style type
fn marker_for_style(list_style_type: ListStyleType, index: usize) -> Option<Marker> {
    if list_style_type == ListStyleType::None {
        return None;
    }

    Some(match list_style_type {
        ListStyleType::LowerAlpha => {
            let mut marker = String::new();
            build_alpha_marker(index, &mut marker);
            Marker::String(format!("{marker}. "))
        }
        ListStyleType::UpperAlpha => {
            let mut marker = String::new();
            build_alpha_marker(index, &mut marker);
            Marker::String(format!("{}. ", marker.to_ascii_uppercase()))
        }
        ListStyleType::Decimal => Marker::String(format!("{}. ", index + 1)),
        ListStyleType::Disc => Marker::Char('•'),
        ListStyleType::Circle => Marker::Char('◦'),
        ListStyleType::Square => Marker::Char('▪'),
        ListStyleType::DisclosureOpen => Marker::Char('▾'),
        ListStyleType::DisclosureClosed => Marker::Char('▸'),
        _ => Marker::Char('□'),
    })
}

const ALPHABET: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

// Construct alphanumeric marker from index, appending characters when index exceeds powers of 26
fn build_alpha_marker(index: usize, str: &mut String) {
    let rem = index % 26;
    let sym = ALPHABET[rem];
    str.insert(0, sym);
    let rest = (index - rem) as i64 / 26 - 1;
    if rest >= 0 {
        build_alpha_marker(rest as usize, str);
    }
}

#[test]
fn test_marker_for_disc() {
    let result = marker_for_style(ListStyleType::Disc, 0);
    assert_eq!(result, Some(Marker::Char('•')));
}

#[test]
fn test_marker_for_decimal() {
    let result_1 = marker_for_style(ListStyleType::Decimal, 0);
    let result_2 = marker_for_style(ListStyleType::Decimal, 1);
    assert_eq!(result_1, Some(Marker::String("1. ".to_string())));
    assert_eq!(result_2, Some(Marker::String("2. ".to_string())));
}

#[test]
fn test_marker_for_lower_alpha() {
    let result_1 = marker_for_style(ListStyleType::LowerAlpha, 0);
    let result_2 = marker_for_style(ListStyleType::LowerAlpha, 1);
    let result_extended_1 = marker_for_style(ListStyleType::LowerAlpha, 26);
    let result_extended_2 = marker_for_style(ListStyleType::LowerAlpha, 27);
    assert_eq!(result_1, Some(Marker::String("a. ".to_string())));
    assert_eq!(result_2, Some(Marker::String("b. ".to_string())));
    assert_eq!(result_extended_1, Some(Marker::String("aa. ".to_string())));
    assert_eq!(result_extended_2, Some(Marker::String("ab. ".to_string())));
}

#[test]
fn test_marker_for_upper_alpha() {
    let result_1 = marker_for_style(ListStyleType::UpperAlpha, 0);
    let result_2 = marker_for_style(ListStyleType::UpperAlpha, 1);
    let result_extended_1 = marker_for_style(ListStyleType::UpperAlpha, 26);
    let result_extended_2 = marker_for_style(ListStyleType::UpperAlpha, 27);
    assert_eq!(result_1, Some(Marker::String("A. ".to_string())));
    assert_eq!(result_2, Some(Marker::String("B. ".to_string())));
    assert_eq!(result_extended_1, Some(Marker::String("AA. ".to_string())));
    assert_eq!(result_extended_2, Some(Marker::String("AB. ".to_string())));
}

/// Handles the cases where there are text nodes or inline nodes that need to be wrapped in an anonymous block node
fn collect_complex_layout_children(
    doc: &mut BaseDocument,
    container_node_id: usize,
    layout_children: &mut Vec<usize>,
    anonymous_block_id: &mut Option<usize>,
    hide_whitespace: bool,
    needs_wrap: impl Fn(NodeKind, DisplayOutside) -> bool,
) {
    fn block_is_only_whitespace(doc: &BaseDocument, node_id: usize) -> bool {
        for child_id in doc.nodes[node_id].children.iter().copied() {
            let child = &doc.nodes[child_id];
            if child
                .text_data()
                .is_none_or(|text_data| !text_data.content.chars().all(|c| c.is_ascii_whitespace()))
            {
                return false;
            }
        }

        true
    }

    doc.iter_children_and_pseudos_mut(container_node_id, |child_id, doc| {
        // Get node kind (text, element, comment, etc)
        let child_node_kind = doc.nodes[child_id].data.kind();

        // Get Display style. Default to inline because nodes without styles are probably text nodes
        let contains_block = doc.nodes[child_id].is_or_contains_block();
        let child_display = &doc.nodes[child_id]
            .display_style()
            .unwrap_or(Display::inline());
        let display_inside = child_display.inside();
        let display_outside = if contains_block {
            DisplayOutside::Block
        } else {
            child_display.outside()
        };

        let is_whitespace_node = match &doc.nodes[child_id].data {
            NodeData::Text(data) => data.content.chars().all(|c| c.is_ascii_whitespace()),
            _ => false,
        };

        // Skip comment nodes. Note that we do *not* skip `Display::None` nodes as they may need to be hidden.
        // Taffy knows how to deal with `Display::None` children.
        //
        // Also hide all-whitespace flexbox children as these should be ignored
        if child_node_kind == NodeKind::Comment || (hide_whitespace && is_whitespace_node) {
            // return;
        }
        // Recurse into `Display::Contents` nodes
        else if display_inside == DisplayInside::Contents {
            collect_layout_children(doc, child_id, layout_children, anonymous_block_id)
        }
        // Push nodes that need wrapping into the current "anonymous block container".
        // If there is not an open one then we create one.
        else if needs_wrap(child_node_kind, display_outside) {
            use style::selector_parser::PseudoElement;

            if anonymous_block_id.is_none() {
                const NAME: QualName = QualName {
                    prefix: None,
                    ns: ns!(html),
                    local: local_name!("div"),
                };
                let node_id =
                    doc.create_node(NodeData::AnonymousBlock(ElementData::new(NAME, Vec::new())));

                // Set style data
                let parent_style = match doc.nodes[container_node_id].primary_styles() {
                    Some(style) => style,
                    None => {
                        eprintln!("Warning: Container node {} has no primary styles for anonymous block creation", container_node_id);
                        return;
                    }
                };
                let read_guard = doc.guard.read();
                let guards = StylesheetGuards::same(&read_guard);
                let style = doc.stylist.style_for_anonymous::<&Node>(
                    &guards,
                    &PseudoElement::ServoAnonymousBox,
                    &parent_style,
                );
                let mut stylo_element_data = StyloElementData::default();
                stylo_element_data.styles.primary = Some(style);
                stylo_element_data.set_restyled();
                *doc.nodes[node_id].stylo_element_data.borrow_mut() = Some(stylo_element_data);

                layout_children.push(node_id);
                *anonymous_block_id = Some(node_id);
            }

            match *anonymous_block_id {
                Some(anon_id) => {
                    doc.nodes[anon_id].children.push(child_id);
                }
                None => {
                    eprintln!("Warning: Anonymous block ID is None when trying to add child {}", child_id);
                    return;
                }
            }
        }
        // Else push the child directly (and close any open "anonymous block container")
        else {
            // If anonymous block node only contains whitespace then delete it
            if let Some(anon_id) = *anonymous_block_id
                && block_is_only_whitespace(doc, anon_id)
            {
                layout_children.pop();
                doc.nodes.remove(anon_id);
            }

            *anonymous_block_id = None;
            layout_children.push(child_id);
        }
    });

    // If anonymous block node only contains whitespace then delete it
    if let Some(anon_id) = *anonymous_block_id
        && block_is_only_whitespace(doc, anon_id)
    {
        layout_children.pop();
        doc.nodes.remove(anon_id);
    }
}

fn create_text_editor(doc: &mut BaseDocument, input_element_id: usize, is_multiline: bool) {
    let node = &mut doc.nodes[input_element_id];
    let cosmyc_style = node
        .primary_styles()
        .as_ref()
        .map(|s| stylo_to_blitz::style(input_element_id, s))
        .unwrap_or_else(|| crate::layout::stylo_to_blitz::CosmicStyle::default());

    let element = match node.data.downcast_element_mut() {
        Some(element) => element,
        None => {
            eprintln!(
                "Warning: Cannot create text editor for node {}: node is not an element",
                input_element_id
            );
            return;
        }
    };

    if !matches!(element.special_data, SpecialElementData::TextInput(_)) {
        // Create text input with cosmyc-text using the shared font_system
        let mut text_input_data = doc
            .text_system
            .with_font_system(|font_system| TextInputData::new(font_system, is_multiline));

        // Set text content with styling
        let text_content = element.attr(local_name!("value")).unwrap_or(" ");
        doc.text_system.with_font_system(|font_system| {
            text_input_data.editor.with_buffer_mut(|buffer| {
                buffer.set_text(
                    font_system,
                    text_content,
                    &cosmyc_style.attrs.as_attrs(),
                    blitz_text::Shaping::Advanced,
                );

                // Set buffer properties
                buffer.set_wrap(font_system, cosmyc_style.wrap);
                buffer.set_metrics(font_system, cosmyc_style.metrics);

                // Set width if specified (cosmyc-text uses finite dimensions)
                let scale = doc.viewport.scale() as f32;
                buffer.set_size(font_system, Some(300.0 * scale), Some(f32::INFINITY));

                // Shape the buffer to create layout runs needed for rendering
                buffer.shape_until_scroll(font_system, false);
            });

            // Shape the editor's buffer - ensures text is properly laid out for rendering
            // Edit import removed - functionality is inherent to editor
            text_input_data.editor.shape_as_needed(font_system, true);
        });

        element.special_data = SpecialElementData::TextInput(text_input_data);
    }
}

fn create_checkbox_input(doc: &mut BaseDocument, input_element_id: usize) {
    let node = &mut doc.nodes[input_element_id];

    let element = match node.data.downcast_element_mut() {
        Some(element) => element,
        None => {
            eprintln!(
                "Warning: Cannot create checkbox input for node {}: node is not an element",
                input_element_id
            );
            return;
        }
    };
    if !matches!(element.special_data, SpecialElementData::CheckboxInput(_)) {
        let checked = element.has_attr(local_name!("checked"));
        element.special_data = SpecialElementData::CheckboxInput(checked);
    }
}

pub(crate) fn build_inline_layout(
    doc: &mut BaseDocument,
    inline_context_root_node_id: usize,
) -> (TextLayout, Vec<usize>) {
    // println!("Inline context {}", inline_context_root_node_id);

    flush_inline_pseudos_recursive(doc, inline_context_root_node_id);

    // Get the inline context's root node's text styles
    let root_node = &doc.nodes[inline_context_root_node_id];
    let root_node_style = root_node.primary_styles().or_else(|| {
        root_node
            .parent
            .and_then(|parent_id| doc.nodes[parent_id].primary_styles())
    });

    let cosmyc_style = root_node_style
        .as_ref()
        .map(|s| stylo_to_blitz::style(inline_context_root_node_id, s))
        .unwrap_or_else(|| crate::layout::stylo_to_blitz::CosmicStyle::default());

    // dbg!(&cosmyc_style);

    let _root_line_height = resolve_line_height(
        cosmyc_style.metrics.line_height,
        cosmyc_style.metrics.font_size,
    );

    // Create cosmyc-text buffer for inline layout
    let mut buffer = doc.text_system.with_font_system(|font_system| {
        let mut buffer = blitz_text::EnhancedBuffer::new(font_system, cosmyc_style.metrics);
        buffer.set_wrap_cached(font_system, cosmyc_style.wrap);
        buffer
    });

    // Extract white-space-collapse mode from computed styles for CSS compliance
    let collapse_mode = stylo_to_blitz::white_space_collapse_to_mode(
        root_node
            .primary_styles()
            .map(|styles| styles.get_inherited_text().clone_white_space_collapse())
            .unwrap_or(
                style::properties::longhands::white_space_collapse::computed_value::T::Collapse,
            ),
    );

    // Track text content for building the buffer
    let mut text_content = String::new();

    // Render position-inside list items
    if let Some(ListItemLayout {
        marker,
        position: ListItemLayoutPosition::Inside,
    }) = root_node
        .element_data()
        .and_then(|el| el.list_item_data.as_deref())
    {
        match marker {
            Marker::Char(char) => text_content.push_str(&format!("{char} ")),
            Marker::String(str) => text_content.push_str(str),
        }
    };

    // Collect text content from all child nodes
    if let Some(before_id) = root_node.before {
        collect_inline_text_recursive(&mut text_content, &doc.nodes, before_id, collapse_mode);
    }
    for child_id in root_node.children.iter().copied() {
        collect_inline_text_recursive(&mut text_content, &doc.nodes, child_id, collapse_mode);
    }
    if let Some(after_id) = root_node.after {
        collect_inline_text_recursive(&mut text_content, &doc.nodes, after_id, collapse_mode);
    }

    // Set the collected text in the buffer with styling
    doc.text_system.with_font_system(|font_system| {
        buffer.set_text_cached(
            font_system,
            &text_content,
            &cosmyc_style.attrs.as_attrs(),
            blitz_text::Shaping::Advanced,
        );
    });

    // Obtain layout children for the inline layout
    let mut layout_children: Vec<usize> = Vec::new();

    // Include ALL original DOM children (text nodes, inline elements, etc.)
    let root_node = &doc.nodes[inline_context_root_node_id];
    if let Some(before) = root_node.before {
        layout_children.push(before);
    }
    layout_children.extend_from_slice(&root_node.children);
    if let Some(after) = root_node.after {
        layout_children.push(after);
    }

    return (
        TextLayout {
            text: text_content,
            layout: buffer,
            inline_boxes: Vec::new(), // Empty inline boxes for this case
            cached_content_widths: None,
            cached_text_hash: None,
        },
        layout_children,
    );

    fn flush_inline_pseudos_recursive(doc: &mut BaseDocument, node_id: usize) {
        doc.iter_children_mut(node_id, |child_id, doc| {
            flush_pseudo_elements(doc, child_id);
            let display = doc.nodes[node_id]
                .display_style()
                .unwrap_or(Display::inline());
            let do_recurse = match (display.outside(), display.inside()) {
                (DisplayOutside::None, DisplayInside::Contents) => true,
                (DisplayOutside::Inline, DisplayInside::Flow) => true,
                (_, _) => false,
            };
            if do_recurse {
                flush_inline_pseudos_recursive(doc, child_id);
            }
        });
    }

    // Note: build_inline_layout_recursive has been replaced with collect_inline_text_recursive
    // in the collect_inline_text.rs file. This provides better separation of concerns and
    // simplifies the text collection process for cosmyc-text buffers.
}
