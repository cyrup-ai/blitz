use blitz_text::Edit;
use blitz_text::text_system::Action;
use blitz_traits::{
    events::{
        BlitzInputEvent, BlitzMouseButtonEvent, DomEvent, DomEventData, MouseEventButton,
        MouseEventButtons,
    },
    navigation::NavigationOptions,
};
use markup5ever::local_name;

use crate::{BaseDocument, node::SpecialElementData};

pub(crate) fn handle_mousemove(
    doc: &mut BaseDocument,
    target: usize,
    x: f32,
    y: f32,
    buttons: MouseEventButtons,
) -> bool {
    let mut changed = doc.set_hover_to(x, y);

    let Some(hit) = doc.hit(x, y) else {
        return changed;
    };

    if hit.node_id != target {
        return changed;
    }

    // First, extract the needed layout and attribute data
    let (content_box_offset, disabled, has_text_input) = {
        let node = &doc.nodes[target];
        let Some(el) = node.data.downcast_element() else {
            return changed;
        };

        let disabled = el.attr(local_name!("disabled")).is_some();
        let has_text_input = matches!(el.special_data, SpecialElementData::TextInput(_));
        let content_box_offset = taffy::Point {
            x: node.final_layout.padding.left + node.final_layout.border.left,
            y: node.final_layout.padding.top + node.final_layout.border.top,
        };

        (content_box_offset, disabled, has_text_input)
    };

    if disabled || !has_text_input {
        return changed;
    }

    if buttons == MouseEventButtons::None {
        return changed;
    }

    let x = (hit.x - content_box_offset.x) as f64 * doc.viewport.scale_f64();
    let y = (hit.y - content_box_offset.y) as f64 * doc.viewport.scale_f64();

    // Use the new API that safely handles both text system and nodes
    doc.with_text_and_nodes(|text_system, nodes| {
        text_system.with_font_system(|font_system| {
            let node = &mut nodes[target];
            let Some(el) = node.data.downcast_element_mut() else {
                return;
            };
            if let SpecialElementData::TextInput(ref mut text_input_data) = el.special_data {
                let mut editor_borrowed = text_input_data.editor.borrow_with(font_system);
                editor_borrowed.action(Action::Drag {
                    x: x as i32,
                    y: y as i32,
                });
            }
        });
    });

    changed = true;
    changed
}

pub(crate) fn handle_mousedown(doc: &mut BaseDocument, target: usize, x: f32, y: f32) {
    let Some(hit) = doc.hit(x, y) else {
        return;
    };
    if hit.node_id != target {
        return;
    }

    // First, extract the needed layout and attribute data
    let (content_box_offset, disabled, has_text_input) = {
        let node = &doc.nodes[target];
        let Some(el) = node.data.downcast_element() else {
            return;
        };

        let disabled = el.attr(local_name!("disabled")).is_some();
        let has_text_input = matches!(el.special_data, SpecialElementData::TextInput(_));
        let content_box_offset = taffy::Point {
            x: node.final_layout.padding.left + node.final_layout.border.left,
            y: node.final_layout.padding.top + node.final_layout.border.top,
        };

        (content_box_offset, disabled, has_text_input)
    };

    if disabled || !has_text_input {
        return;
    }

    let x = (hit.x - content_box_offset.x) as f64 * doc.viewport.scale_f64();
    let y = (hit.y - content_box_offset.y) as f64 * doc.viewport.scale_f64();

    // Use the new API that safely handles both text system and nodes
    doc.with_text_and_nodes(|text_system, nodes| {
        text_system.with_font_system(|font_system| {
            let node = &mut nodes[target];
            let Some(el) = node.data.downcast_element_mut() else {
                return;
            };
            if let SpecialElementData::TextInput(ref mut text_input_data) = el.special_data {
                let mut editor_borrowed = text_input_data.editor.borrow_with(font_system);
                editor_borrowed.action(Action::Click {
                    x: x as i32,
                    y: y as i32,
                });
            }
        });
    });

    doc.set_focus_to(hit.node_id);
}

pub(crate) fn handle_mouseup<F: FnMut(DomEvent)>(
    doc: &mut BaseDocument,
    target: usize,
    event: &BlitzMouseButtonEvent,
    mut dispatch_event: F,
) {
    if doc.devtools().highlight_hover {
        let mut node = match doc.get_node(target) {
            Some(node) => node,
            None => {
                eprintln!(
                    "Warning: Cannot highlight hover for node {}: node not found",
                    target
                );
                return;
            }
        };
        if event.button == MouseEventButton::Secondary
            && let Some(parent_id) = node.layout_parent.get()
        {
            node = match doc.get_node(parent_id) {
                Some(node) => node,
                None => {
                    eprintln!(
                        "Warning: Cannot find parent node {} for hover highlighting",
                        parent_id
                    );
                    // Keep using the current node as fallback
                    node
                }
            };
        }
        doc.debug_log_node(node.id);
        doc.devtools_mut().highlight_hover = false;
        return;
    }

    // Determine whether to dispatch a click event
    let do_click = true;
    // let do_click = doc.mouse_down_node.is_some_and(|mouse_down_id| {
    //     // Anonymous node ids are unstable due to tree reconstruction. So we compare the id
    //     // of the first non-anonymous ancestor.
    //     mouse_down_id == target
    //         || doc.non_anon_ancestor_if_anon(mouse_down_id) == doc.non_anon_ancestor_if_anon(target)
    // });

    // Dispatch a click event
    if do_click && event.button == MouseEventButton::Main {
        dispatch_event(DomEvent::new(target, DomEventData::Click(event.clone())));
    }
}

pub(crate) fn handle_click<F: FnMut(DomEvent)>(
    doc: &mut BaseDocument,
    target: usize,
    event: &BlitzMouseButtonEvent,
    mut dispatch_event: F,
) {
    let mut maybe_node_id = Some(target);
    while let Some(node_id) = maybe_node_id {
        let maybe_element = {
            let node = &mut doc.nodes[node_id];
            node.data.downcast_element_mut()
        };

        let Some(el) = maybe_element else {
            maybe_node_id = doc.nodes[node_id].parent;
            continue;
        };

        let disabled = el.attr(local_name!("disabled")).is_some();
        if disabled {
            return;
        }

        if let SpecialElementData::TextInput(_) = el.special_data {
            return;
        } else if el.name.local == local_name!("input")
            && matches!(el.attr(local_name!("type")), Some("checkbox"))
        {
            let is_checked = BaseDocument::toggle_checkbox(el);
            let value = is_checked.to_string();
            dispatch_event(DomEvent::new(
                node_id,
                DomEventData::Input(BlitzInputEvent { value }),
            ));
            doc.set_focus_to(node_id);
            return;
        } else if el.name.local == local_name!("input")
            && matches!(el.attr(local_name!("type")), Some("radio"))
        {
            let radio_set = match el.attr(local_name!("name")) {
                Some(name) => name.to_string(),
                None => {
                    eprintln!(
                        "Warning: Radio input node {} has no name attribute, skipping radio toggle",
                        node_id
                    );
                    return;
                }
            };
            // TODO: make input event conditional on value actually changing
            let value = el
                .attr(local_name!("value"))
                .unwrap_or("on")
                .to_string();
                
            BaseDocument::toggle_radio(doc, radio_set, node_id);
            dispatch_event(DomEvent::new(
                node_id,
                DomEventData::Input(BlitzInputEvent { value }),
            ));

            doc.set_focus_to(node_id);

            return;
        }
        // Clicking labels triggers click, and possibly input event, of associated input
        else if el.name.local == local_name!("label") {
            if let Some(target_node_id) = doc.label_bound_input_element(node_id).map(|n| n.id) {
                // Apply default click event action for target node
                let target_node = match doc.get_node_mut(target_node_id) {
                    Some(node) => node,
                    None => {
                        eprintln!(
                            "Warning: Cannot find target input node {} for label {}",
                            target_node_id, node_id
                        );
                        return;
                    }
                };
                let syn_event = target_node.synthetic_click_event_data(event.mods);
                handle_click(doc, target_node_id, &syn_event, dispatch_event);
                return;
            }
        } else if el.name.local == local_name!("a") {
            if let Some(href) = el.attr(local_name!("href")) {
                if let Some(url) = doc.url.resolve_relative(href) {
                    doc.navigation_provider.navigate_to(NavigationOptions::new(
                        url,
                        String::from("text/plain"),
                        doc.id(),
                    ));
                } else {
                    println!("{href} is not parseable as a url. : {:?}", *doc.url)
                }
                return;
            } else {
                println!("Clicked link without href: {:?}", el.attrs());
            }
        } else if (el.name.local == local_name!("input")
            && el.attr(local_name!("type")) == Some("submit")
            || el.name.local == local_name!("button"))
            && let Some(form_owner) = doc.controls_to_form.get(&node_id)
        {
            doc.submit_form(*form_owner, node_id);
        } else if el.name.local == local_name!("input")
            && el.attr(local_name!("type")) == Some("image")
            && let Some(form_owner) = doc.controls_to_form.get(&node_id)
        {
            // Use existing hit detection for element-relative coordinates
            if let Some(hit) = doc.hit(event.x, event.y) {
                doc.submit_form_with_coordinates(
                    *form_owner,
                    node_id,
                    Some((hit.x as i32, hit.y as i32)),
                );
            } else {
                eprintln!(
                    "Warning: Click on image button {} has no hit result",
                    node_id
                );
                // Fallback to submitting without coordinates
                doc.submit_form(*form_owner, node_id);
            }
            return;
        }

        // No match. Recurse up to parent.
        maybe_node_id = doc.nodes[node_id].parent;
    }

    // If nothing is matched then clear focus
    doc.clear_focus();
}
