//! Action handling and motion processing for enhanced editor
//!
//! This module implements action processing, cursor motion handling,
//! and user interaction management for the enhanced editor.

use std::sync::atomic::Ordering;

use cosmyc_text::{Action, Cursor, Edit, FontSystem, Motion, Selection};

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
            Edit::delete_selection(editor);
            let cursor = editor.cursor();
            editor.insert_at(cursor, &ch.to_string(), None);
        }
        Action::Enter => {
            Edit::delete_selection(editor);
            let cursor = editor.cursor();
            let line_ending = if editor.auto_indent {
                get_auto_indent_string(editor)
            } else {
                "\n".to_string()
            };
            editor.insert_at(cursor, &line_ending, None);
        }
        Action::Backspace => {
            if Edit::delete_selection(editor) {
                // Deleted selection
            } else {
                use unicode_segmentation::UnicodeSegmentation;
                
                let end = editor.cursor();
                let mut start = end;

                if start.index > 0 {
                    // Move cursor to previous grapheme cluster
                    start.index = editor.with_buffer(|buffer| {
                        if start.line < buffer.lines.len() {
                            buffer.lines[start.line].text()[..start.index]
                                .grapheme_indices(true)
                                .next_back()
                                .map_or(0, |(i, _)| i)
                        } else {
                            0
                        }
                    });
                } else if start.line > 0 {
                    // Join with previous line
                    start.line -= 1;
                    start.index = editor.with_buffer(|buffer| {
                        if start.line < buffer.lines.len() {
                            buffer.lines[start.line].text().len()
                        } else {
                            0
                        }
                    });
                }

                if start != end {
                    editor.delete_range(start, end);
                    editor.set_cursor(start);
                }
            }
        }
        Action::Delete => {
            if Edit::delete_selection(editor) {
                // Deleted selection
            } else {
                use unicode_segmentation::UnicodeSegmentation;
                
                let mut start = editor.cursor();
                let mut end = start;

                editor.with_buffer(|buffer| {
                    if start.line < buffer.lines.len() {
                        if start.index < buffer.lines[start.line].text().len() {
                            let line = &buffer.lines[start.line];

                            // Find next grapheme cluster boundary
                            let range_opt = line
                                .text()
                                .grapheme_indices(true)
                                .take_while(|(i, _)| *i <= start.index)
                                .last()
                                .map(|(i, c)| i..(i + c.len()));

                            if let Some(range) = range_opt {
                                start.index = range.start;
                                end.index = range.end;
                            }
                        } else if start.line + 1 < buffer.lines.len() {
                            // Join with next line
                            end.line += 1;
                            end.index = 0;
                        }
                    }
                });

                if start != end {
                    editor.delete_range(start, end);
                    editor.set_cursor(start);
                }
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
            if let Some(()) = editor.with_buffer_mut(|buffer| {
                buffer.set_scroll(cosmyc_text::Scroll::new(
                    buffer.scroll().line.saturating_add_signed(lines as isize),
                    buffer.scroll().vertical,
                    buffer.scroll().horizontal,
                ));
            }) {}
        }
        Action::Indent => {
            handle_indent(editor);
        }
        Action::Unindent => {
            handle_unindent(editor);
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

    // Analyze current line indentation and preserve it
    editor.with_buffer(|buffer| {
        let cursor = editor.cursor();
        
        if cursor.line >= buffer.lines.len() {
            return "\n".to_string();
        }

        let line = &buffer.lines[cursor.line];
        let text = line.text();
        
        // Collect leading whitespace
        let mut indent = String::from("\n");
        for ch in text.chars() {
            if ch == ' ' || ch == '\t' {
                indent.push(ch);
            } else {
                break;
            }
        }
        
        indent
    })
}

/// Handle indent action
fn handle_indent<'buffer>(editor: &mut EnhancedEditor<'buffer>) {
    let tab_width = editor.tab_width as usize;
    let tab_string = " ".repeat(tab_width);
    
    // Get selection bounds or current cursor
    let (start_line, end_line) = match editor.selection() {
        Selection::None => {
            // No selection - insert tab at cursor
            let cursor = editor.cursor();
            editor.insert_at(cursor, &tab_string, None);
            return;
        }
        Selection::Normal(sel_cursor) | Selection::Word(sel_cursor) | Selection::Line(sel_cursor) => {
            let cursor = editor.cursor();
            let start = cursor.line.min(sel_cursor.line);
            let end = cursor.line.max(sel_cursor.line);
            (start, end)
        }
    };

    // Calculate indent positions for all selected lines
    let indent_positions: Vec<(usize, usize)> = editor.with_buffer(|buffer| {
        let num_lines = buffer.lines.len();
        let actual_end = end_line.min(num_lines.saturating_sub(1));
        
        let mut positions = Vec::new();
        for line_i in start_line..=actual_end {
            if line_i < num_lines {
                let line = &buffer.lines[line_i];
                let text = line.text();
                
                let indent_pos = text
                    .char_indices()
                    .find(|(_, c)| !c.is_whitespace())
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());
                
                positions.push((line_i, indent_pos));
            }
        }
        positions
    });

    for (line_i, pos) in indent_positions.iter().rev() {
        editor.insert_at(Cursor::new(*line_i, *pos), &tab_string, None);
    }

    // Adjust cursor only if it was on an indented line
    let cursor = editor.cursor();
    if let Some((_, indent_pos)) = indent_positions.iter().find(|(line, _)| *line == cursor.line) {
        // Only adjust if cursor is at or after the indent position
        if cursor.index >= *indent_pos {
            editor.set_cursor(Cursor::new(cursor.line, cursor.index.saturating_add(tab_width)));
        }
    }
}

/// Handle unindent action
fn handle_unindent<'buffer>(editor: &mut EnhancedEditor<'buffer>) {
    let tab_width = editor.tab_width as usize;
    
    // Get selection bounds or current cursor
    let (start_line, end_line) = match editor.selection() {
        Selection::None => {
            let cursor = editor.cursor();
            (cursor.line, cursor.line)
        }
        Selection::Normal(sel_cursor) | Selection::Word(sel_cursor) | Selection::Line(sel_cursor) => {
            let cursor = editor.cursor();
            let start = cursor.line.min(sel_cursor.line);
            let end = cursor.line.max(sel_cursor.line);
            (start, end)
        }
    };

    // Calculate unindent ranges for all selected lines
    let unindent_ranges: Vec<(usize, usize, usize)> = editor.with_buffer(|buffer| {
        let num_lines = buffer.lines.len();
        if start_line >= num_lines {
            return Vec::new();
        }
        
        let actual_end = end_line.min(num_lines.saturating_sub(1));
        
        let mut ranges = Vec::new();
        for line_i in start_line..=actual_end {
            if line_i < num_lines {
                let line = &buffer.lines[line_i];
                let text = line.text();
                
                // Count leading whitespace to remove
                let mut remove_count = 0;
                let mut remove_end = 0;
                
                for (idx, ch) in text.char_indices() {
                    if ch == ' ' {
                        remove_count += 1;
                        remove_end = idx + 1;
                        if remove_count >= tab_width {
                            break;
                        }
                    } else if ch == '\t' {
                        remove_count = tab_width;
                        remove_end = idx + 1;
                        break;
                    } else {
                        break;
                    }
                }
                
                if remove_count > 0 {
                    ranges.push((line_i, 0, remove_end));
                }
            }
        }
        ranges
    });

    // Apply unindents (in reverse order to maintain indices)
    for (line_i, start_idx, end_idx) in unindent_ranges.iter().rev() {
        let start_cursor = Cursor::new(*line_i, *start_idx);
        let end_cursor = Cursor::new(*line_i, *end_idx);
        editor.delete_range(start_cursor, end_cursor);
    }

    // Adjust cursor
    let cursor = editor.cursor();
    let removed = if let Some((_, _, end_idx)) = unindent_ranges.iter().find(|(line, _, _)| *line == cursor.line) {
        *end_idx
    } else {
        0
    };
    editor.set_cursor(Cursor::new(cursor.line, cursor.index.saturating_sub(removed)));
}
