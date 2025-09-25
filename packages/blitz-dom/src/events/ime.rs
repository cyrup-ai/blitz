use std::{cell::RefCell, collections::HashMap};

use blitz_text::{Cursor, Edit, text_system::Action};
use blitz_traits::events::BlitzImeEvent;

use crate::BaseDocument;

// Thread-local IME composition state tracker for managing preedit text
thread_local! {
    static COMPOSITION_STATE: RefCell<HashMap<usize, CompositionInfo>> = RefCell::new(HashMap::new());
}

#[derive(Debug, Clone)]
struct CompositionInfo {
    preedit_text: String,
    preedit_start: usize,
    preedit_cursor: Option<(usize, usize)>,
}

pub(crate) fn handle_ime_event(doc: &mut BaseDocument, event: BlitzImeEvent) {
    if let Some(node_id) = doc.focus_node_id {
        let node = &mut doc.nodes[node_id];
        let text_input_data = node
            .data
            .downcast_element_mut()
            .and_then(|el| el.text_input_data_mut());
        if let Some(input_data) = text_input_data {
            let _editor = &mut input_data.editor;

            match event {
                BlitzImeEvent::Enabled => { /* Do nothing */ }
                BlitzImeEvent::Disabled => {
                    // Clear any active composition state
                    COMPOSITION_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        if let Some(composition) = state.remove(&node_id) {
                            // Clear existing preedit text by selecting and deleting it
                            let preedit_end =
                                composition.preedit_start + composition.preedit_text.len();
                            _editor.set_cursor(Cursor::new(0, composition.preedit_start));
                            _editor.set_selection(blitz_text::text_system::Selection::Normal(
                                Cursor::new(0, preedit_end),
                            ));
                            // Action methods need font_system parameter
                            doc.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                                _editor.action(font_system, Action::Delete);
                            }));

                            // Restore cursor position from stored preedit_cursor if available
                            if let Some((cursor_start, _cursor_end)) = composition.preedit_cursor {
                                let final_cursor_pos = composition.preedit_start
                                    + cursor_start.min(composition.preedit_text.len());
                                _editor.set_cursor(Cursor::new(0, final_cursor_pos));
                            }
                        }
                    });

                    // Ensure text is properly shaped after clearing composition
                    doc.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                        _editor.shape_as_needed(font_system, false);
                    });

                    // Request redraw to update display
                    doc.shell_provider.request_redraw();
                }
                BlitzImeEvent::Commit(text) => {
                    // Replace any existing preedit text with committed text
                    COMPOSITION_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        if let Some(composition) = state.remove(&node_id) {
                            // Replace preedit text with committed text
                            let preedit_end =
                                composition.preedit_start + composition.preedit_text.len();
                            _editor.set_cursor(Cursor::new(0, composition.preedit_start));
                            _editor.set_selection(blitz_text::text_system::Selection::Normal(
                                Cursor::new(0, preedit_end),
                            ));
                            // Action methods need font_system parameter
                            doc.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                                _editor.action(font_system, Action::Delete);
                            }));

                            // Use stored preedit_cursor for insertion positioning context
                            if let Some((_cursor_start, cursor_end)) = composition.preedit_cursor {
                                // Cursor end position provides context for where the committed text should be placed
                                // This ensures proper text flow continuation after composition
                                let cursor_context_pos = composition.preedit_start
                                    + cursor_end.min(composition.preedit_text.len());
                                _editor.set_cursor(Cursor::new(0, cursor_context_pos));
                            }
                        }
                    });

                    // Insert committed text with cosmyc-text editor
                    _editor.insert_string(&text, None);

                    // Ensure text is properly shaped after insertion
                    doc.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                        _editor.shape_as_needed(font_system, false);
                    });

                    // Request redraw to show the new text
                    doc.shell_provider.request_redraw();
                }
                BlitzImeEvent::Preedit(text, cursor) => {
                    // Handle preedit text with cosmyc-text editor
                    COMPOSITION_STATE.with(|state| {
                        let mut state = state.borrow_mut();

                        // Clear any existing preedit text
                        if let Some(composition) = state.get(&node_id) {
                            let preedit_end =
                                composition.preedit_start + composition.preedit_text.len();
                            _editor.set_cursor(Cursor::new(0, composition.preedit_start));
                            _editor.set_selection(blitz_text::text_system::Selection::Normal(
                                Cursor::new(0, preedit_end),
                            ));
                            // Action methods need font_system parameter
                            doc.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                                _editor.action(font_system, Action::Delete);
                            }));
                        }

                        if text.is_empty() {
                            // Clear composition when text is empty
                            state.remove(&node_id);
                        } else {
                            // Insert new preedit text
                            let current_cursor = _editor.cursor();
                            let preedit_start = current_cursor.index;

                            // Insert the preedit text
                            _editor.insert_string(&text, None);

                            // Set cursor position within preedit if specified
                            if let Some((start, end)) = cursor {
                                let cursor_pos = preedit_start + start.min(text.len());
                                // Set cursor position directly
                                _editor.set_cursor(Cursor::new(current_cursor.line, cursor_pos));

                                // If there's a selection range in the preedit, set it
                                if start != end {
                                    let selection_end = preedit_start + end.min(text.len());
                                    _editor.set_selection(
                                        blitz_text::text_system::Selection::Normal(Cursor::new(
                                            current_cursor.line,
                                            selection_end,
                                        )),
                                    );
                                }
                            }

                            // Store composition info for later cleanup
                            state.insert(
                                node_id,
                                CompositionInfo {
                                    preedit_text: text.to_string(),
                                    preedit_start,
                                    preedit_cursor: cursor,
                                },
                            );
                        }
                    });

                    // Ensure text is properly shaped after composition changes
                    doc.with_text_system(|text_system| text_system.with_font_system(|font_system| {
                        _editor.shape_as_needed(font_system, false);
                    });

                    // Request redraw to show preedit text
                    doc.shell_provider.request_redraw();
                }
            }
            println!("Sent ime event to {node_id}");
        }
    }
}
