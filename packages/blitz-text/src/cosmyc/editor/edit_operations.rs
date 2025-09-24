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
            self.with_buffer_mut(|buffer| {
                buffer.set_tab_width(font_system, tab_width);
            });
        }
    }

    fn shape_as_needed(&mut self, font_system: &mut FontSystem, prune: bool) {
        self.with_buffer_mut(|buffer| {
            buffer.shape_until_scroll(font_system, prune);
        });
    }

    fn delete_range(&mut self, _start: Cursor, _end: Cursor) {
        let start_change = Change { items: vec![] };
        self.start_change();

        // Implementation would delete text between cursors
        // This is a simplified version - full implementation would need
        // to handle line boundaries, character clusters, etc.

        let change = self.finish_change().unwrap_or(start_change);
        self.push_change(change);
        self.delete_operations.fetch_add(1, Ordering::Relaxed);
    }

    fn insert_at(&mut self, cursor: Cursor, data: &str, _attrs_list: Option<AttrsList>) -> Cursor {
        self.start_change();

        // Implementation would insert text at cursor position
        // This is a simplified version - full implementation would need
        // to handle proper text insertion with attributes

        let change_item = ChangeItem {
            start: cursor,
            end: cursor,
            text: data.to_string(),
            insert: true,
        };

        let change = Change {
            items: vec![change_item],
        };

        self.push_change(change);
        self.insert_operations.fetch_add(1, Ordering::Relaxed);

        // Return new cursor position after insertion
        Cursor::new(cursor.line, cursor.index + data.len())
    }

    fn copy_selection(&self) -> Option<String> {
        if let Some((_start, _end)) = self.selection_bounds() {
            // Implementation would extract text between selection bounds
            // This is a simplified version
            Some(String::new())
        } else {
            None
        }
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
                // Insert text
                self.insert_at(item.start, &item.text);
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
        self.with_buffer(|buffer| buffer.hit(0.0, 0.0).map(|_| (0, 0)))
    }
}

impl<'buffer> EnhancedEditor<'buffer> {
    /// Push change to undo stack
    pub(super) fn push_change(&mut self, change: Change) {
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
