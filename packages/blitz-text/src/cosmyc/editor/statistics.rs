//! Statistics tracking and reporting for enhanced editor
//!
//! This module provides statistics collection, undo/redo functionality,
//! and performance monitoring for the enhanced editor.

use std::sync::atomic::Ordering;

use cosmyc_text::Edit;

use super::types::{EditorStats, EnhancedEditor};

impl<'buffer> EnhancedEditor<'buffer> {
    /// Undo last change
    pub fn undo(&mut self) -> bool {
        if let Some(mut change) = self.undo_stack.pop() {
            // Set flag to prevent re-adding to undo stack
            self.applying_change = true;
            
            // Reverse the change
            change.reverse();

            // Apply reversed change
            self.apply_change(&change);

            // Reverse back for redo
            change.reverse();
            
            // Add to redo stack
            self.redo_stack.push(change);
            
            // Clear flag
            self.applying_change = false;
            
            self.undo_operations.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Redo last undone change
    pub fn redo(&mut self) -> bool {
        if let Some(change) = self.redo_stack.pop() {
            // Set flag to prevent re-adding to undo stack
            self.applying_change = true;
            
            // Apply change
            self.apply_change(&change);

            // Add back to undo stack
            self.undo_stack.push(change);
            
            // Clear flag
            self.applying_change = false;
            
            self.redo_operations.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get editor statistics
    pub fn get_stats(&self) -> EditorStats {
        EditorStats {
            total_actions: self.total_actions.load(Ordering::Relaxed),
            insert_operations: self.insert_operations.load(Ordering::Relaxed),
            delete_operations: self.delete_operations.load(Ordering::Relaxed),
            motion_operations: self.motion_operations.load(Ordering::Relaxed),
            undo_operations: self.undo_operations.load(Ordering::Relaxed),
            redo_operations: self.redo_operations.load(Ordering::Relaxed),
            undo_stack_depth: self.undo_stack.len(),
            redo_stack_depth: self.redo_stack.len(),
        }
    }

    /// Clear all statistics
    pub fn clear_stats(&self) {
        self.total_actions.store(0, Ordering::Relaxed);
        self.insert_operations.store(0, Ordering::Relaxed);
        self.delete_operations.store(0, Ordering::Relaxed);
        self.motion_operations.store(0, Ordering::Relaxed);
        self.undo_operations.store(0, Ordering::Relaxed);
        self.redo_operations.store(0, Ordering::Relaxed);
    }
}
