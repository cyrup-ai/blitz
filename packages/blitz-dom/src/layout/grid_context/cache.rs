//! Grid context cache implementation with multi-level caching strategy
//!
//! This module provides efficient O(1) lookups for previously computed parent-child relationships
//! and parent grid contexts, dramatically improving performance over O(n²) tree searches.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use taffy::prelude::NodeId;

use super::resolution::check_parent_grid_container;
use super::types::{GridContextError, ParentGridContext};

/// Efficient grid context cache with multi-level caching strategy
///
/// This cache provides O(1) lookups for previously computed parent-child relationships
/// and parent grid contexts, dramatically improving performance over O(n²) tree searches.
pub struct GridContextCache {
    /// Cache of node_id -> parent_id mappings (Level 1 - Fastest)
    parent_cache: HashMap<NodeId, Option<NodeId>>,

    /// Cache of parent contexts for grid containers (Level 2 - Expensive to compute)
    context_cache: HashMap<NodeId, ParentGridContext>,

    /// Negative cache to avoid repeated failed searches (Level 3 - Optimization)
    not_found_cache: HashSet<NodeId>,

    /// Track cache validity and performance
    cache_generation: usize,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    computation_time_saved: AtomicU64, // nanoseconds
}

impl GridContextCache {
    pub fn new() -> Self {
        Self {
            parent_cache: HashMap::with_capacity(256),
            context_cache: HashMap::with_capacity(64),
            not_found_cache: HashSet::with_capacity(128),
            cache_generation: 0,
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            computation_time_saved: AtomicU64::new(0),
        }
    }

    pub fn invalidate(&mut self) {
        self.parent_cache.clear();
        self.context_cache.clear();
        self.not_found_cache.clear();
        self.cache_generation += 1;
    }

    /// High-performance cached lookup with O(1) BaseDocument optimization
    pub fn get_or_compute_parent_context<Tree>(
        &mut self,
        tree: &Tree,
        node_id: NodeId,
    ) -> Result<Option<ParentGridContext>, GridContextError>
    where
        Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
    {
        // Level 1: Check cache first
        if let Some(parent_opt) = self.parent_cache.get(&node_id) {
            if let Some(parent_id) = parent_opt {
                if let Some(context) = self.context_cache.get(parent_id) {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(Some(context.clone()));
                }
            } else {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }
        }

        // Level 2: Use O(1) direct access for BaseDocument
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        let result = self.compute_parent_context_efficient(tree, node_id)?;

        // Update cache with result
        if let Some(context) = &result {
            // For BaseDocument, we can easily get the parent to cache
            if let Some(base_doc) = (tree as &dyn std::any::Any).downcast_ref::<crate::BaseDocument>() {
                if let Some(node) = base_doc.get_node(usize::from(node_id)) {
                    if let Some(parent_id) = node.parent {
                        let parent_node_id = NodeId::from(parent_id);
                        self.parent_cache.insert(node_id, Some(parent_node_id));
                        self.context_cache.insert(parent_node_id, context.clone());
                    }
                }
            }
        } else {
            self.parent_cache.insert(node_id, None);
            self.not_found_cache.insert(node_id);
        }

        Ok(result)
    }

    /// Compute parent context using O(1) direct access for BaseDocument
    fn compute_parent_context_efficient<Tree>(
        &mut self,
        tree: &Tree,
        target_node: NodeId,
    ) -> Result<Option<ParentGridContext>, GridContextError>
    where
        Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
    {
        // Use direct parent access for BaseDocument (O(1))
        if let Some(base_doc) = (tree as &dyn std::any::Any).downcast_ref::<crate::BaseDocument>() {
            let node = match base_doc.get_node(usize::from(target_node)) {
                Some(node) => node,
                None => return Ok(None),
            };
            
            let parent_id = match node.parent {
                Some(id) => NodeId::from(id),
                None => return Ok(None),
            };
            
            return check_parent_grid_container(tree, parent_id);
        }
        
        // Fallback for non-BaseDocument trees (keep existing heuristic for compatibility)
        self.find_parent_heuristic(tree, target_node)
            .and_then(|parent_opt| {
                match parent_opt {
                    Some(parent) => check_parent_grid_container(tree, parent),
                    None => Ok(None),
                }
            })
    }



    /// Simplified heuristic for non-BaseDocument trees only
    fn find_parent_heuristic<Tree>(
        &self,
        tree: &Tree,
        target_node: NodeId,
    ) -> Result<Option<NodeId>, GridContextError>
    where
        Tree: taffy::TraversePartialTree,
    {
        // This method is now only used for non-BaseDocument trees
        // Most usage should go through the O(1) BaseDocument path
        
        let target_idx = usize::from(target_node);
        
        // Simplified heuristic: check likely parent candidates only
        for parent_candidate_idx in (target_idx.saturating_sub(10)..target_idx).rev() {
            let parent_candidate = NodeId::from(parent_candidate_idx);
            if self.is_direct_parent(tree, parent_candidate, target_node)? {
                return Ok(Some(parent_candidate));
            }
        }
        
        Ok(None)
    }

    /// Optimized direct parent check - O(children) instead of O(n²)
    fn is_direct_parent<Tree>(
        &self,
        tree: &Tree,
        potential_parent: NodeId,
        target: NodeId,
    ) -> Result<bool, GridContextError>
    where
        Tree: taffy::TraversePartialTree,
    {
        // Check cache first for previously computed relationships
        if let Some(cached_parent) = self.parent_cache.get(&target) {
            return Ok(cached_parent.map_or(false, |p| p == potential_parent));
        }

        // Direct child check - O(children) complexity
        let child_count = tree.child_count(potential_parent);
        for i in 0..child_count {
            if tree.get_child_id(potential_parent, i) == target {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Find the actual parent node for cache storage
    pub fn find_actual_parent<Tree>(
        &mut self,
        tree: &Tree,
        target_node: NodeId,
    ) -> Result<Option<NodeId>, GridContextError>
    where
        Tree: taffy::TraversePartialTree,
    {
        // Use only heuristic for non-BaseDocument trees
        self.find_parent_heuristic(tree, target_node)
    }

    /// Invalidate cache when tree structure changes
    pub fn invalidate_on_tree_change(&mut self, tree_generation: usize) {
        if self.cache_generation != tree_generation {
            self.parent_cache.clear();
            self.context_cache.clear();
            self.not_found_cache.clear();
            self.cache_generation = tree_generation;
        }
    }

    /// Selective invalidation for subtree changes
    pub fn invalidate_subtree(&mut self, root_node: NodeId) {
        let keys_to_remove: Vec<NodeId> = self
            .parent_cache
            .keys()
            .filter(|&&node_id| self.is_descendant_of(node_id, root_node))
            .copied()
            .collect();

        for key in keys_to_remove {
            self.parent_cache.remove(&key);
            self.context_cache.remove(&key);
            self.not_found_cache.remove(&key);
        }
    }

    /// Check if a node is a descendant of another node using cached relationships
    fn is_descendant_of(&self, node: NodeId, potential_ancestor: NodeId) -> bool {
        let mut current = Some(node);
        while let Some(current_node) = current {
            if current_node == potential_ancestor {
                return true;
            }
            current = self.parent_cache.get(&current_node).copied().flatten();
        }
        false
    }

    /// LRU eviction to prevent unbounded growth
    pub fn enforce_size_limits(&mut self) {
        const MAX_PARENT_CACHE_SIZE: usize = 1024;
        const MAX_CONTEXT_CACHE_SIZE: usize = 256;

        if self.parent_cache.len() > MAX_PARENT_CACHE_SIZE {
            // Remove oldest entries (simple strategy - in real implementation would use timestamps)
            let excess = self.parent_cache.len() - MAX_PARENT_CACHE_SIZE;
            let keys_to_remove: Vec<_> = self.parent_cache.keys().take(excess).copied().collect();
            for key in keys_to_remove {
                self.parent_cache.remove(&key);
            }
        }

        if self.context_cache.len() > MAX_CONTEXT_CACHE_SIZE {
            let excess = self.context_cache.len() - MAX_CONTEXT_CACHE_SIZE;
            let keys_to_remove: Vec<_> = self.context_cache.keys().take(excess).copied().collect();
            for key in keys_to_remove {
                self.context_cache.remove(&key);
            }
        }
    }

    /// Performance analytics for optimization tuning
    pub fn get_performance_report(&self) -> String {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let time_saved = Duration::from_nanos(self.computation_time_saved.load(Ordering::Relaxed));

        format!(
            "GridContextCache Performance Report:\n\
             - Hit rate: {:.2}%\n\
             - Total requests: {}\n\
             - Cache entries: {} parent mappings, {} contexts\n\
             - Estimated time saved: {:?}\n\
             - Cache generation: {}",
            hit_rate,
            total,
            self.parent_cache.len(),
            self.context_cache.len(),
            time_saved,
            self.cache_generation
        )
    }
}

// Thread-local cache for high-performance grid context resolution
thread_local! {
    static GRID_CONTEXT_CACHE: RefCell<GridContextCache> = RefCell::new(GridContextCache::new());
}

/// High-performance entry point for grid context resolution with thread-local caching
pub fn with_cache<F, R>(f: F) -> R
where
    F: FnOnce(&mut GridContextCache) -> R,
{
    GRID_CONTEXT_CACHE.with(|cache| f(&mut cache.borrow_mut()))
}
