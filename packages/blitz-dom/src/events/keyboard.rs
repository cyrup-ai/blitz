// Edit import removed - use blitz_text re-exports
use blitz_text::{Edit, UnifiedTextSystem};
use blitz_traits::{
    events::{BlitzInputEvent, BlitzKeyEvent, DomEvent, DomEventData},
    shell::ShellProvider,
};
use keyboard_types::{Key, Modifiers, NamedKey};
use markup5ever::local_name;

// FontContext and LayoutContext replaced with blitz-text UnifiedTextSystem
use crate::{BaseDocument, node::TextInputData};

#[derive(Debug, Clone)]
enum GeneratedEvent {
    Input,
    Submit,
    Blur,
    KeyDown,
    KeyUp,
    KeyPress, // Character-producing key events following web standards
    EscapeRevert, // Revert value to original and trigger Blur (HTML Living Standard)
}

pub(crate) fn handle_keypress<F: FnMut(DomEvent)>(
    doc: &mut BaseDocument,
    target: usize,
    event: BlitzKeyEvent,
    mut dispatch_event: F,
) {
    // Handle Tab navigation (both forward and reverse)
    if event.key == Key::Named(NamedKey::Tab) {
        // Generate Change and Blur events for the currently focused element before focus moves
        // According to HTML Living Standard: Change events fire before Blur when focus moves
        if let Some(current_focus_target) = doc.focus_node_id {
            // Tab navigation commits any pending changes before moving focus
            // Change event fires first ONLY if the input value has been modified (HTML standards-compliant)
            if let Some(input_data) = doc.nodes[current_focus_target].element_data()
                .and_then(|ed| ed.text_input_data()) {
                if input_data.has_value_changed() {
                    trigger_change_event(current_focus_target, &mut dispatch_event);
                }
            }
            // Blur event fires second as focus leaves the element
            trigger_blur_event(current_focus_target, &mut dispatch_event);
        }
        
        if event.modifiers.contains(Modifiers::SHIFT) {
            // Shift+Tab for reverse tab navigation
            doc.focus_previous_node();
        } else {
            doc.focus_next_node();
        }
        
        // Generate Focus event for the newly focused element and capture original value
        if let Some(new_focus_target) = doc.focus_node_id {
            // Capture original value for HTML standards-compliant Change event detection
            if let Some(input_data) = doc.nodes[new_focus_target].element_data_mut()
                .and_then(|ed| ed.text_input_data_mut()) {
                input_data.capture_original_value();
            }
            trigger_focus_event(new_focus_target, dispatch_event);
        }
        return;
    }

    if let Some(node_id) = doc.focus_node_id {
        if target != node_id {
            return;
        }

        let node = &mut doc.nodes[node_id];
        let Some(element_data) = node.element_data_mut() else {
            return;
        };

        if let Some(input_data) = element_data.text_input_data_mut() {
            let event_clone = event.clone();
            let generated_event = apply_keypress_event(
                input_data,
                &mut doc.text_system,
                &*doc.shell_provider,
                event,
            );

            if let Some(generated_event) = generated_event {
                match generated_event {
                    GeneratedEvent::Input => {
                        // Get text from cosmyc-text Editor by accessing the buffer
                        let value = input_data.editor.with_buffer(|buffer| {
                            buffer
                                .lines
                                .iter()
                                .map(|line| line.text())
                                .collect::<Vec<_>>()
                                .join("\n")
                        });
                        dispatch_event(DomEvent::new(
                            node_id,
                            DomEventData::Input(BlitzInputEvent { value }),
                        ));
                    }
                    GeneratedEvent::Submit => {
                        // Generate submit event that can be handled by script
                        dispatch_event(DomEvent::new(target, DomEventData::Submit));
                        // Also perform implicit form submission for compatibility
                        implicit_form_submission(doc, target);
                    }
                    GeneratedEvent::Blur => {
                        if doc.focus_node_id == Some(target) {
                            doc.focus_node_id = None;
                        }
                        trigger_blur_event(target, &mut dispatch_event);
                    }
                    GeneratedEvent::KeyDown => {
                        trigger_keydown_event(target, event_clone.clone(), &mut dispatch_event);
                    }
                    GeneratedEvent::KeyUp => {
                        trigger_keyup_event(target, event_clone.clone(), &mut dispatch_event);
                    }
                    GeneratedEvent::KeyPress => {
                        // KeyPress events for character-producing keys (web standards-compliant)
                        trigger_keypress_event(target, event_clone.clone(), &mut dispatch_event);
                        
                        // Also generate Input event since character was inserted
                        if let Some(input_data) = doc.nodes[target].element_data()
                            .and_then(|ed| ed.text_input_data()) {
                            let value = input_data.get_current_value();
                            dispatch_event(DomEvent::new(
                                target,
                                DomEventData::Input(BlitzInputEvent { value }),
                            ));
                        }
                    }
                    GeneratedEvent::EscapeRevert => {
                        // HTML Living Standard Escape key behavior:
                        // 1. Revert input value to original state
                        if let Some(input_data) = doc.nodes[target].element_data_mut()
                            .and_then(|ed| ed.text_input_data_mut()) {
                            doc.text_system.with_font_system(|font_system| {
                                input_data.revert_to_original_value(font_system);
                            });
                        }
                        // 2. Remove focus and trigger Blur (no Change event since value is reverted)
                        if doc.focus_node_id == Some(target) {
                            doc.focus_node_id = None;
                        }
                        trigger_blur_event(target, &mut dispatch_event);
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
const ACTION_MOD: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
const ACTION_MOD: Modifiers = Modifiers::CONTROL;

fn apply_keypress_event(
    input_data: &mut TextInputData,
    text_system: &mut UnifiedTextSystem,
    shell_provider: &dyn ShellProvider,
    event: BlitzKeyEvent,
) -> Option<GeneratedEvent> {
    // Generate KeyUp events for key release
    if !event.state.is_pressed() {
        return Some(GeneratedEvent::KeyUp);
    }

    let mods = event.modifiers;
    let _shift = mods.contains(Modifiers::SHIFT);
    let action_mod = mods.contains(ACTION_MOD);

    let is_multiline = input_data.is_multiline;
    let editor = &mut input_data.editor;

    // Access underlying FontSystem through blitz-text's lock-free interface
    text_system.with_font_system(|font_system| {
        let mut editor_borrowed = editor.borrow_with(font_system);

        match event.key {
            Key::Named(NamedKey::Tab) if _shift => {
                // Shift+Tab (reverse tab) moves focus away, handled at function level
                // This should not be reached due to early return in handle_keypress
                None
            }
            Key::Character(c) if action_mod && matches!(c.as_str(), "c" | "x" | "v") => {
                match c.to_lowercase().as_str() {
                    "c" => {
                        // Copy selected text to clipboard
                        if let Some(selected_text) = editor_borrowed.copy_selection() {
                            if !selected_text.is_empty() {
                                let _ = shell_provider.set_clipboard_text(selected_text);
                            }
                        }
                    }
                    "x" => {
                        // Cut selected text to clipboard
                        if let Some(selected_text) = editor_borrowed.copy_selection() {
                            if !selected_text.is_empty() {
                                let _ = shell_provider.set_clipboard_text(selected_text);
                                editor_borrowed.delete_selection();
                            }
                        }
                    }
                    "v" => {
                        // Paste text from clipboard
                        if let Ok(text) = shell_provider.get_clipboard_text() {
                            for ch in text.chars() {
                                editor_borrowed.action(blitz_text::Action::Insert(ch));
                            }
                        }
                    }
                    _ => unreachable!(),
                }

                Some(GeneratedEvent::Input)
            }
            Key::Character(c) if action_mod && matches!(c.to_lowercase().as_str(), "a") => {
                // Select all text
                let cursor = editor_borrowed.cursor();
                editor_borrowed.set_selection(blitz_text::Selection::Line(cursor));
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::ArrowLeft) => {
                use blitz_text::Motion;
                if action_mod {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::PreviousWord));
                } else {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::Left));
                }
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::ArrowRight) => {
                use blitz_text::Motion;
                if action_mod {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::NextWord));
                } else {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::Right));
                }
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::ArrowUp) => {
                use blitz_text::Motion;
                editor_borrowed.action(blitz_text::Action::Motion(Motion::Up));
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::ArrowDown) => {
                use blitz_text::Motion;
                editor_borrowed.action(blitz_text::Action::Motion(Motion::Down));
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::Home) => {
                use blitz_text::Motion;
                if action_mod {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::BufferStart));
                } else {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::Home));
                }
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::End) => {
                use blitz_text::Motion;
                if action_mod {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::BufferEnd));
                } else {
                    editor_borrowed.action(blitz_text::Action::Motion(Motion::End));
                }
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::Delete) => {
                editor_borrowed.action(blitz_text::Action::Delete);
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::Backspace) => {
                editor_borrowed.action(blitz_text::Action::Backspace);
                Some(GeneratedEvent::Input)
            }
            Key::Named(NamedKey::Enter) => {
                // Enter key behavior according to HTML Living Standard:
                // - In multiline inputs (textarea): Insert newline, trigger Input event
                // - In single-line inputs: Commit value and submit form, trigger Submit event
                if is_multiline {
                    editor_borrowed.action(blitz_text::Action::Enter);
                    Some(GeneratedEvent::Input)
                } else {
                    // Single-line input: Enter commits the value and triggers form submission
                    Some(GeneratedEvent::Submit)
                }
            }

            Key::Character(s) => {
                // Character-producing keys generate KeyPress events (web standards-compliant)
                // Insert each character from the string
                for ch in s.chars() {
                    editor_borrowed.action(blitz_text::Action::Insert(ch));
                }
                // Generate both KeyPress and Input events for character-producing keys
                Some(GeneratedEvent::KeyPress)
            }

            Key::Named(NamedKey::Escape) => {
                // Escape key behavior according to HTML Living Standard:
                // 1. Revert input value to original state (cancels any uncommitted changes)
                // 2. Remove focus from the element (triggers Blur)
                // 3. No Change event is fired since value is reverted
                Some(GeneratedEvent::EscapeRevert)
            }
            _ => {
                // For any other key press, generate a KeyDown event
                Some(GeneratedEvent::KeyDown)
            }
        }
    })
}

/// Generate focus event when an element programmatically receives focus
pub(crate) fn trigger_focus_event<F: FnMut(DomEvent)>(
    target: usize,
    mut dispatch_event: F,
) {
    dispatch_event(DomEvent::new(target, DomEventData::Focus));
}

/// Generate change event when an input element value has been modified and committed
pub(crate) fn trigger_change_event<F: FnMut(DomEvent)>(
    target: usize,
    mut dispatch_event: F,
) {
    dispatch_event(DomEvent::new(target, DomEventData::Change));
}

/// Generate blur event when an element loses focus
pub(crate) fn trigger_blur_event<F: FnMut(DomEvent)>(
    target: usize,
    mut dispatch_event: F,
) {
    dispatch_event(DomEvent::new(target, DomEventData::Blur));
}

/// Generate keydown event when a key is pressed down
pub(crate) fn trigger_keydown_event<F: FnMut(DomEvent)>(
    target: usize,
    event: BlitzKeyEvent,
    mut dispatch_event: F,
) {
    dispatch_event(DomEvent::new(target, DomEventData::KeyDown(event)));
}

/// Generate keyup event when a key is released
pub(crate) fn trigger_keyup_event<F: FnMut(DomEvent)>(
    target: usize,
    event: BlitzKeyEvent,
    mut dispatch_event: F,
) {
    dispatch_event(DomEvent::new(target, DomEventData::KeyUp(event)));
}

/// Generate keypress event for character-producing keys (web standards-compliant)
pub(crate) fn trigger_keypress_event<F: FnMut(DomEvent)>(
    target: usize,
    event: BlitzKeyEvent,
    mut dispatch_event: F,
) {
    dispatch_event(DomEvent::new(target, DomEventData::KeyPress(event)));
}


/// https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#field-that-blocks-implicit-submission
fn implicit_form_submission(doc: &BaseDocument, text_target: usize) {
    let Some(form_owner_id) = doc.controls_to_form.get(&text_target) else {
        return;
    };
    if doc
        .controls_to_form
        .iter()
        .filter(|(_control_id, form_id)| *form_id == form_owner_id)
        .filter_map(|(control_id, _)| doc.nodes[*control_id].element_data())
        .filter(|element_data| {
            element_data.attr(local_name!("type")).is_some_and(|t| {
                matches!(
                    t,
                    "text"
                        | "search"
                        | "email"
                        | "url"
                        | "tel"
                        | "password"
                        | "date"
                        | "month"
                        | "week"
                        | "time"
                        | "datetime-local"
                        | "number"
                )
            })
        })
        .count()
        > 1
    {
        return;
    }

    doc.submit_form(*form_owner_id, *form_owner_id);
}
