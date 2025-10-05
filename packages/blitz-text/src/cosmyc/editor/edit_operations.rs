//! Core editing operations and Edit trait implementation
//!
//! This module implements the Edit trait for EnhancedEditor, providing
//! fundamental text editing capabilities including insertion, deletion,
//! and buffer management.

use std::sync::atomic::Ordering;

use cosmyc_text::{AttrsList, BufferRef, Change, ChangeItem, Cursor, Edit, FontSystem, Selection};

use super::types::EnhancedEditor;

impl<'buffer> Edit<'buffer> for EnhancedEditor<'buffer> {
    fn buffer_ref(&self) -> &BufferRef<'buffer> {
        &self.buffer
    }

    fn buffer_ref_mut(&mut self) -> &mut BufferRef<'buffer> {
        &mut self.buffer
    }

    fn cursor(&self) -> Cursor {
        self.cursor
    }

    fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

    fn selection(&self) -> Selection {
        self.selection
    }

    fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
    }

    fn auto_indent(&self) -> bool {
        self.auto_indent
    }

    fn set_auto_indent(&mut self, auto_indent: bool) {
        self.auto_indent = auto_indent;
    }

    fn tab_width(&self) -> u16 {
        self.tab_width
    }

    fn set_tab_width(&mut self, font_system: &mut FontSystem, tab_width: u16) {
        if tab_width > 0 {
            self.tab_width = tab_width;
            // Update buffer tab width
            if let Some(()) = self.with_buffer_mut(|buffer| {
                buffer.set_tab_width(font_system, tab_width);
            }) {}
        }
    }

    fn shape_as_needed(&mut self, font_system: &mut FontSystem, prune: bool) {
        if let Some(()) = self.with_buffer_mut(|buffer| {
            buffer.shape_until_scroll(font_system, prune);
        }) {}
    }

    fn delete_range(&mut self, start: Cursor, end: Cursor) {
        // Normalize cursor order
        let (start, end) = if start.line > end.line || (start.line == end.line && start.index > end.index) {
            (end, start)
        } else {
            (start, end)
        };

        // Only track changes if not applying undo/redo
        let should_track = !self.applying_change;
        
        if should_track {
            self.start_change();
        }

        let change_item = match Edit::with_buffer_mut(self, |buffer| -> Option<ChangeItem> {
            // Validate cursors
            if start.line >= buffer.lines.len() || end.line >= buffer.lines.len() {
                return None;
            }

            // Validate indices
            if start.line < buffer.lines.len() && start.index > buffer.lines[start.line].text().len() {
                return None;
            }
            if end.line < buffer.lines.len() && end.index > buffer.lines[end.line].text().len() {
                return None;
            }

            let mut change_lines = Vec::new();

            // Step 1: Handle end line (if different from start line)
            let end_line_opt = if end.line > start.line {
                let after = buffer.lines[end.line].split_off(end.index);
                let removed = buffer.lines.remove(end.line);
                change_lines.insert(0, removed.text().to_string());
                Some(after)
            } else {
                None
            };

            // Step 2: Remove intermediate lines (in reverse for safety)
            for line_i in (start.line + 1..end.line).rev() {
                let removed = buffer.lines.remove(line_i);
                change_lines.insert(0, removed.text().to_string());
            }

            // Step 3: Handle start line
            {
                // Get part after selection if same line
                let after_opt = if start.line == end.line {
                    Some(buffer.lines[start.line].split_off(end.index))
                } else {
                    None
                };

                // Delete selected part
                let removed = buffer.lines[start.line].split_off(start.index);
                change_lines.insert(0, removed.text().to_string());

                // Re-add parts after selection
                if let Some(after) = after_opt {
                    buffer.lines[start.line].append(after);
                }
                if let Some(end_line) = end_line_opt {
                    buffer.lines[start.line].append(end_line);
                }
            }

            Some(ChangeItem {
                start,
                end,
                text: change_lines.join("\n"),
                insert: false,
                attrs: None,
            })
        }) {
            Some(item) => item,
            None => ChangeItem {
                start,
                end,
                text: String::new(),
                insert: false,
                attrs: None,
            }
        };

        if should_track {
            self.change_history.push(change_item);
            let change = self.finish_change().unwrap_or_else(|| Change { items: vec![] });
            self.push_change(change);
        }
        
        self.delete_operations.fetch_add(1, Ordering::Relaxed);
    }

    fn insert_at(&mut self, mut cursor: Cursor, data: &str, attrs_list: Option<AttrsList>) -> Cursor {
        if data.is_empty() {
            return cursor;
        }

        // Only track changes if not applying undo/redo
        let should_track = !self.applying_change;
        
        if should_track {
            self.start_change();
        }

        let (change_item, new_cursor) = if let Some(result) = self.with_buffer_mut(|buffer| {
            use cosmyc_text::{Attrs, BufferLine, Shaping};

            let start = cursor;

            // Step 1: Ensure buffer has enough lines
            while cursor.line >= buffer.lines.len() {
                let ending = buffer
                    .lines
                    .last()
                    .map(|line| line.ending())
                    .unwrap_or_default();
                let line = BufferLine::new(
                    String::new(),
                    ending,
                    cosmyc_text::AttrsList::new(&attrs_list.as_ref().map_or_else(
                        || Attrs::new(),
                        |x| x.defaults(),
                    )),
                    Shaping::Advanced,
                );
                buffer.lines.push(line);
            }

            // Validate cursor index
            if cursor.line < buffer.lines.len() && cursor.index > buffer.lines[cursor.line].text().len() {
                cursor.index = buffer.lines[cursor.line].text().len();
            }

            // Step 2: Prepare for insertion
            let line: &mut BufferLine = &mut buffer.lines[cursor.line];
            let insert_line = cursor.line + 1;
            let ending = line.ending();
            let after: BufferLine = line.split_off(cursor.index);
            let after_len = after.text().len();

            // Step 3: Prepare attributes
            let attrs_for_change = attrs_list.clone();
            let mut final_attrs = attrs_list.unwrap_or_else(|| {
                cosmyc_text::AttrsList::new(
                    &line.attrs_list().get_span(cursor.index.saturating_sub(1)),
                )
            });

            // Step 4: Insert text line by line
            let mut lines_iter = data.split_inclusive('\n');

            // First line appends to current line
            if let Some(data_line) = lines_iter.next() {
                let mut these_attrs = final_attrs.split_off(data_line.len());
                std::mem::swap(&mut these_attrs, &mut final_attrs);
                line.append(BufferLine::new(
                    data_line
                        .strip_suffix(char::is_control)
                        .unwrap_or(data_line),
                    ending,
                    these_attrs,
                    Shaping::Advanced,
                ));
            }

            // Last line (if exists) contains the "after" content
            if let Some(data_line) = lines_iter.next_back() {
                let mut tmp = BufferLine::new(
                    data_line
                        .strip_suffix(char::is_control)
                        .unwrap_or(data_line),
                    ending,
                    final_attrs.split_off(data_line.len()),
                    Shaping::Advanced,
                );
                tmp.append(after);
                buffer.lines.insert(insert_line, tmp);
                cursor.line += 1;
            } else {
                line.append(after);
            }

            // Middle lines
            for data_line in lines_iter.rev() {
                let tmp = BufferLine::new(
                    data_line
                        .strip_suffix(char::is_control)
                        .unwrap_or(data_line),
                    ending,
                    final_attrs.split_off(data_line.len()),
                    Shaping::Advanced,
                );
                buffer.lines.insert(insert_line, tmp);
                cursor.line += 1;
            }

            // Update cursor position
            cursor.index = buffer.lines[cursor.line].text().len() - after_len;

            let change_item = ChangeItem {
                start,
                end: cursor,
                text: data.to_string(),
                insert: true,
                attrs: attrs_for_change,
            };

            (change_item, cursor)
        }) {
            result
        } else {
            // Buffer operation failed, return original cursor
            let change_item = ChangeItem {
                start: cursor,
                end: cursor,
                text: data.to_string(),
                insert: true,
                attrs: None,
            };
            (change_item, cursor)
        };

        if should_track {
            self.change_history.push(change_item);
            let change = self
                .finish_change()
                .unwrap_or_else(|| Change { items: vec![] });
            self.push_change(change);
        }
        
        self.insert_operations.fetch_add(1, Ordering::Relaxed);

        new_cursor
    }

    fn copy_selection(&self) -> Option<String> {
        let (start, end) = self.selection_bounds()?;
        
        self.with_buffer(|buffer| {
            // Validate cursors
            if start.line >= buffer.lines.len() || end.line >= buffer.lines.len() {
                return None;
            }

            let mut selection = String::new();
            
            // Take the selection from the first line
            {
                if start.line == end.line {
                    let line_text = buffer.lines[start.line].text();
                    let end_idx = end.index.min(line_text.len());
                    let start_idx = start.index.min(end_idx);
                    selection.push_str(&line_text[start_idx..end_idx]);
                } else {
                    let line_text = buffer.lines[start.line].text();
                    let start_idx = start.index.min(line_text.len());
                    selection.push_str(&line_text[start_idx..]);
                    selection.push('\n');
                }
            }

            // Take the selection from all interior lines (if they exist)
            for line_i in start.line + 1..end.line {
                if line_i < buffer.lines.len() {
                    selection.push_str(buffer.lines[line_i].text());
                    selection.push('\n');
                }
            }

            // Take the selection from the last line
            if end.line > start.line && end.line < buffer.lines.len() {
                let line_text = buffer.lines[end.line].text();
                let end_idx = end.index.min(line_text.len());
                selection.push_str(&line_text[..end_idx]);
            }

            Some(selection)
        })
    }

    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_bounds() {
            self.delete_range(start, end);
            self.selection = Selection::None;
            true
        } else {
            false
        }
    }

    fn apply_change(&mut self, change: &Change) -> bool {
        // Apply each change item in the change
        for item in &change.items {
            if item.insert {
                // Insert text - restore with original attributes
                self.insert_at(item.start, &item.text, item.attrs.clone());
            } else {
                // Delete text
                self.delete_range(item.start, item.end);
            }
        }
        true
    }

    fn start_change(&mut self) {
        // Start collecting changes for undo/redo
        self.change_history.clear();
    }

    fn finish_change(&mut self) -> Option<Change> {
        if !self.change_history.is_empty() {
            let change = Change {
                items: self.change_history.clone(),
            };
            self.change_history.clear();
            Some(change)
        } else {
            None
        }
    }

    fn action(&mut self, font_system: &mut FontSystem, action: cosmyc_text::Action) {
        super::actions::handle_action(self, font_system, action);
    }

    fn cursor_position(&self) -> Option<(i32, i32)> {
        self.with_buffer(|buffer| {
            // Find the visual position of the cursor
            let cursor = self.cursor;
            
            // Validate cursor
            if cursor.line >= buffer.lines.len() {
                return None;
            }

            let line = &buffer.lines[cursor.line];
            
            // Get layout for this line
            let layout_ref = line.layout_opt();
            let layout = layout_ref.as_ref()?;
            
            // Calculate y position by summing line heights of previous lines
            let mut y = 0.0;
            for i in 0..cursor.line {
                if i < buffer.lines.len() {
                    if let Some(prev_layout) = buffer.lines[i].layout_opt().as_ref() {
                        for run in prev_layout.iter() {
                            // Use line_height_opt or calculate from max_ascent + max_descent
                            let line_h = run.line_height_opt.unwrap_or_else(|| {
                                run.max_ascent + run.max_descent
                            });
                            y += line_h;
                        }
                    }
                }
            }
            
            // Find x position by iterating through glyphs
            let mut x = 0.0;
            let line_text = line.text();
            let target_index = cursor.index.min(line_text.len());
            
            // Iterate through layout runs to find cursor position
            for run in layout.iter() {
                for glyph in run.glyphs.iter() {
                    if glyph.start >= target_index {
                        return Some((x as i32, y as i32));
                    }
                    x += glyph.w;
                }
            }
            
            // Cursor is at end of line
            Some((x as i32, y as i32))
        })
    }
}

impl<'buffer> EnhancedEditor<'buffer> {
    /// Push change to undo stack
    pub(super) fn push_change(&mut self, change: Change) {
        // Don't push empty changes
        if change.items.is_empty() {
            return;
        }

        // Clear redo stack when new change is made
        self.redo_stack.clear();

        // Add to undo stack
        self.undo_stack.push(change);

        // Limit undo stack size
        if self.undo_stack.len() > self.max_undo_depth {
            self.undo_stack.remove(0);
        }
    }
}
