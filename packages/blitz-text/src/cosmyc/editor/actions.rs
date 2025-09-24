//! Action handling and motion processing for enhanced editor
//!
//! This module implements action processing, cursor motion handling,
//! and user interaction management for the enhanced editor.

use std::sync::atomic::Ordering;

use cosmyc_text::{Action, Cursor, FontSystem, Motion, Selection};

use super::types::EnhancedEditor;

/// Handle editor actions with enhanced functionality
pub fn handle_action<'buffer>(
    editor: &mut EnhancedEditor<'buffer>,
    font_system: &mut FontSystem,
    action: Action,
) {
    editor.total_actions.fetch_add(1, Ordering::Relaxed);

    match action {
        Action::Motion(motion) => {
            handle_motion(editor, font_system, motion);
            editor.motion_operations.fetch_add(1, Ordering::Relaxed);
        }
        Action::Insert(ch) => {
            editor.delete_selection();
            let cursor = editor.cursor();
            editor.insert_at(cursor, &ch.to_string());
        }
        Action::Enter => {
            editor.delete_selection();
            let cursor = editor.cursor();
            let line_ending = if editor.auto_indent {
                get_auto_indent_string(editor)
            } else {
                "\n".to_string()
            };
            editor.insert_at(cursor, &line_ending);
        }
        Action::Backspace => {
            editor.delete_selection();
            if editor.selection() == Selection::None {
                let cursor = editor.cursor();
                if cursor.index > 0 {
                    let start = Cursor::new(cursor.line, cursor.index - 1);
                    editor.delete_range(start, cursor);
                    editor.set_cursor(start);
                }
            }
        }
        Action::Delete => {
            editor.delete_selection();
            if editor.selection() == Selection::None {
                let cursor = editor.cursor();
                let end = Cursor::new(cursor.line, cursor.index + 1);
                editor.delete_range(cursor, end);
            }
        }
        Action::Escape => {
            editor.set_selection(Selection::None);
        }
        Action::Click { x, y } => {
            if let Some(new_cursor) = hit_test(editor, x as f32, y as f32) {
                editor.set_cursor(new_cursor);
                editor.set_selection(Selection::None);
            }
        }
        Action::Drag { x, y } => {
            if let Some(new_cursor) = hit_test(editor, x as f32, y as f32) {
                let current_selection = editor.selection();
                match current_selection {
                    Selection::None => {
                        editor.set_selection(Selection::Normal(editor.cursor()));
                    }
                    _ => {}
                }
                editor.set_cursor(new_cursor);
            }
        }
        Action::DoubleClick { x, y } => {
            if let Some(cursor) = hit_test(editor, x as f32, y as f32) {
                editor.set_cursor(cursor);
                editor.set_selection(Selection::Word(cursor));
            }
        }
        Action::TripleClick { x, y } => {
            if let Some(cursor) = hit_test(editor, x as f32, y as f32) {
                editor.set_cursor(cursor);
                editor.set_selection(Selection::Line(cursor));
            }
        }
        Action::Scroll { lines } => {
            // Implementation would scroll the buffer
            editor.with_buffer_mut(|buffer| {
                buffer.set_scroll(cosmyc_text::Scroll::new(
                    buffer.scroll().line.saturating_add_signed(lines as isize),
                    buffer.scroll().vertical,
                    buffer.scroll().horizontal,
                ));
            });
        }
        Action::Indent => {
            // Implementation would indent selected lines
            if editor.selection() != Selection::None {
                // Indent selection
            } else {
                // Insert tab at cursor
                let tab_string = " ".repeat(editor.tab_width as usize);
                let cursor = editor.cursor();
                editor.insert_at(cursor, &tab_string);
            }
        }
        Action::Unindent => {
            // Implementation would unindent selected lines
        }
    }
}

/// Handle cursor motion with enhanced functionality
fn handle_motion<'buffer>(
    editor: &mut EnhancedEditor<'buffer>,
    font_system: &mut FontSystem,
    motion: Motion,
) {
    let current_cursor = editor.cursor();

    if let Some(cursor_result) = editor
        .with_buffer_mut(|buffer| buffer.cursor_motion(font_system, current_cursor, None, motion))
    {
        if let Some((new_cursor, _cursor_x)) = cursor_result {
            editor.set_cursor(new_cursor);
        }
    }
}

/// Hit test to find cursor position at coordinates
fn hit_test<'buffer>(editor: &EnhancedEditor<'buffer>, x: f32, y: f32) -> Option<Cursor> {
    editor.with_buffer(|buffer| buffer.hit(x, y))
}

/// Get auto-indent string for new lines
fn get_auto_indent_string<'buffer>(editor: &EnhancedEditor<'buffer>) -> String {
    if !editor.auto_indent {
        return "\n".to_string();
    }

    // Implementation would analyze current line indentation
    // and return appropriate indent string
    "\n".to_string()
}
