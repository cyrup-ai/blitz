//! Core types and data structures for enhanced editor
//!
//! This module defines the main EnhancedEditor struct and related types
//! for comprehensive text editing functionality.

use std::sync::atomic::AtomicUsize;

use cosmyc_text::{BufferRef, Change, ChangeItem, Cursor, Selection};

/// Enhanced Editor wrapper with comprehensive functionality and statistics
pub struct EnhancedEditor<'buffer> {
    pub(super) buffer: BufferRef<'buffer>,
    pub(super) cursor: Cursor,
    pub(super) selection: Selection,
    pub(super) auto_indent: bool,
    pub(super) tab_width: u16,
    pub(super) change_history: Vec<ChangeItem>,
    pub(super) undo_stack: Vec<Change>,
    pub(super) redo_stack: Vec<Change>,
    pub(super) max_undo_depth: usize,
    pub(super) applying_change: bool,

    // Performance statistics
    pub(super) total_actions: AtomicUsize,
    pub(super) insert_operations: AtomicUsize,
    pub(super) delete_operations: AtomicUsize,
    pub(super) motion_operations: AtomicUsize,
    pub(super) undo_operations: AtomicUsize,
    pub(super) redo_operations: AtomicUsize,
}

impl<'buffer> EnhancedEditor<'buffer> {
    /// Create new enhanced editor with buffer
    pub fn new(buffer: BufferRef<'buffer>) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(0, 0),
            selection: Selection::None,
            auto_indent: true,
            tab_width: 4,
            change_history: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_depth: 100,
            applying_change: false,
            total_actions: AtomicUsize::new(0),
            insert_operations: AtomicUsize::new(0),
            delete_operations: AtomicUsize::new(0),
            motion_operations: AtomicUsize::new(0),
            undo_operations: AtomicUsize::new(0),
            redo_operations: AtomicUsize::new(0),
        }
    }

    /// Create new enhanced editor from owned buffer
    pub fn from_buffer(buffer: cosmyc_text::Buffer) -> EnhancedEditor<'static> {
        EnhancedEditor::new(BufferRef::Owned(buffer))
    }

    /// Get current cursor position
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
        self.motion_operations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_actions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get current selection
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Set selection
    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
        self.motion_operations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_actions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Access buffer with mutable reference
    pub fn with_buffer_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut cosmyc_text::Buffer) -> R,
    {
        match &mut self.buffer {
            BufferRef::Owned(ref mut buffer) => Some(f(buffer)),
            BufferRef::Borrowed(_) => None,
            BufferRef::Arc(_) => None,
        }
    }

    /// Access buffer with immutable reference
    pub fn with_buffer<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&cosmyc_text::Buffer) -> R,
    {
        match &self.buffer {
            BufferRef::Owned(ref buffer) => f(buffer),
            BufferRef::Borrowed(buffer) => f(buffer),
            BufferRef::Arc(buffer) => f(buffer),
        }
    }
}

/// Editor performance statistics
#[derive(Debug, Clone, Copy)]
pub struct EditorStats {
    pub total_actions: usize,
    pub insert_operations: usize,
    pub delete_operations: usize,
    pub motion_operations: usize,
    pub undo_operations: usize,
    pub redo_operations: usize,
    pub undo_stack_depth: usize,
    pub redo_stack_depth: usize,
}

impl std::fmt::Display for EditorStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Editor Stats: {} total actions ({} insert, {} delete, {} motion, {} undo, {} redo), undo stack: {}, redo stack: {}",
            self.total_actions,
            self.insert_operations,
            self.delete_operations,
            self.motion_operations,
            self.undo_operations,
            self.redo_operations,
            self.undo_stack_depth,
            self.redo_stack_depth
        )
    }
}
