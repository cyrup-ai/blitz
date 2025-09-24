//! Tree iteration utilities for layout traversal
//!
//! This module provides iteration helpers for traversing the layout tree,
//! including RefCell-based child iteration for BaseDocument.

use std::cell::Ref;

use taffy::prelude::NodeId;

/// Iterator over children stored in a RefCell for BaseDocument
///
/// This provides safe iteration over child node IDs while maintaining
/// the RefCell borrow for the duration of the iteration.
pub struct RefCellChildIter<'a> {
    items: Ref<'a, [usize]>,
    idx: usize,
}

impl<'a> RefCellChildIter<'a> {
    /// Create a new RefCellChildIter from a borrowed slice reference
    pub fn new(items: Ref<'a, [usize]>) -> RefCellChildIter<'a> {
        RefCellChildIter { items, idx: 0 }
    }
}

impl Iterator for RefCellChildIter<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.items.get(self.idx).map(|id| {
            self.idx += 1;
            NodeId::from(*id)
        })
    }
}
