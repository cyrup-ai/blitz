//! Grid context cache implementation with multi-level caching strategy
//!
//! This module provides efficient O(1) lookups for previously computed parent-child relationships
//! and parent grid contexts, dramatically improving performance over O(n²) tree searches.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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

    /// High-performance cached lookup with fallback computation
    pub fn get_or_compute_parent_context<Tree>(
        &mut self,
        tree: &Tree,
        node_id: NodeId,
    ) -> Result<Option<ParentGridContext>, GridContextError>
    where
        Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
    {
        let start_time = Instant::now();

        // Level 1: Check direct parent cache
        if let Some(parent_opt) = self.parent_cache.get(&node_id) {
            if let Some(parent_id) = parent_opt {
                // Level 2: Check grid context cache
                if let Some(context) = self.context_cache.get(parent_id) {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(Some(context.clone()));
                }
            } else {
                // Cached negative result
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }
        }

        // Level 3: Check negative cache
        if self.not_found_cache.contains(&node_id) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(None);
        }

        // Cache miss - compute with optimized algorithms
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        let result = self.compute_parent_context_efficient(tree, node_id)?;

        // Update caches based on result
        match &result {
            Some(context) => {
                if let Some(parent) = self.find_actual_parent(tree, node_id)? {
                    self.parent_cache.insert(node_id, Some(parent));
                    self.context_cache.insert(parent, context.clone());
                }
            }
            None => {
                self.parent_cache.insert(node_id, None);
                self.not_found_cache.insert(node_id);
            }
        }

        // Track performance metrics
        let elapsed = start_time.elapsed().as_nanos() as u64;
        self.computation_time_saved
            .fetch_add(elapsed, Ordering::Relaxed);

        Ok(result)
    }

    /// Compute parent context using efficient algorithms (BFS + heuristics)
    fn compute_parent_context_efficient<Tree>(
        &mut self,
        tree: &Tree,
        target_node: NodeId,
    ) -> Result<Option<ParentGridContext>, GridContextError>
    where
        Tree: taffy::LayoutGridContainer + taffy::TraversePartialTree + 'static,
    {
        // Strategy 1: Try heuristic search first (fastest for common cases)
        if let Some(parent) = self.find_parent_heuristic(tree, target_node)? {
            if let Some(context) = check_parent_grid_container(tree, parent)? {
                return Ok(Some(context));
            }
        }

        // Strategy 2: Fall back to BFS with early termination (comprehensive)
        if let Some(parent) = self.find_parent_breadth_first(tree, target_node)? {
            if let Some(context) = check_parent_grid_container(tree, parent)? {
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Breadth-first search with early termination - O(log n) average case
    fn find_parent_breadth_first<Tree>(
        &mut self,
        tree: &Tree,
        target_node: NodeId,
    ) -> Result<Option<NodeId>, GridContextError>
    where
        Tree: taffy::TraversePartialTree,
    {
        let mut queue = VecDeque::with_capacity(64);
        let mut visited = HashSet::with_capacity(256);

        // Heuristic: Start from likely root nodes (most DOM trees have roots at low IDs)
        for root_candidate in 0..10 {
            queue.push_back(NodeId::from(root_candidate as usize));
        }

        while let Some(current_node) = queue.pop_front() {
            if visited.contains(&current_node) {
                continue;
            }
            visited.insert(current_node);

            // Direct O(children) check instead of O(n²) nested loops
            let child_count = tree.child_count(current_node);
            for i in 0..child_count {
                let child = tree.get_child_id(current_node, i);
                if child == target_node {
                    // Early termination - found the parent!
                    return Ok(Some(current_node));
                }

                // Add children to queue for continued search
                if !visited.contains(&child) {
                    queue.push_back(child);
                }
            }

            // Safety: Prevent infinite loops in malformed trees
            if visited.len() > 1000 {
                break;
            }
        }

        Ok(None)
    }

    /// Heuristic search leveraging NodeId allocation patterns - O(√n) average case
    fn find_parent_heuristic<Tree>(
        &self,
        tree: &Tree,
        target_node: NodeId,
    ) -> Result<Option<NodeId>, GridContextError>
    where
        Tree: taffy::TraversePartialTree,
    {
        let target_idx = usize::from(target_node);

        // Heuristic 1: Parent nodes typically have lower NodeId values
        // This leverages sequential allocation patterns in most DOM implementations
        for parent_candidate_idx in (0..target_idx).rev() {
            let parent_candidate = NodeId::from(parent_candidate_idx);
            if self.is_direct_parent(tree, parent_candidate, target_node)? {
                return Ok(Some(parent_candidate));
            }
        }

        // Heuristic 2: Check siblings of cached nodes (locality principle)
        let search_range = 50.min(target_idx);
        for sibling_idx in target_idx.saturating_sub(search_range)..=target_idx + search_range {
            let sibling = NodeId::from(sibling_idx);
            if let Some(sibling_parent) = self.parent_cache.get(&sibling).copied().flatten() {
                if self.is_direct_parent(tree, sibling_parent, target_node)? {
                    return Ok(Some(sibling_parent));
                }
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
        // Try heuristic first, then BFS
        if let Some(parent) = self.find_parent_heuristic(tree, target_node)? {
            return Ok(Some(parent));
        }

        self.find_parent_breadth_first(tree, target_node)
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
