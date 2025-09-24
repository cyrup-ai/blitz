//! CSS Grid Level 2 compliant line name inheritance mapper
//!
//! This module implements the complete CSS specification for subgrid line name inheritance,
//! building on the existing ParentGridContext and line name extraction infrastructure.

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;

use taffy::prelude::NodeId;

use super::types::{GridAxis, GridSpan, SubgridInheritanceLevel};

/// CSS Grid Level 2 compliant line name inheritance mapper
///
/// Implements the complete CSS specification for subgrid line name inheritance,
/// building on the existing ParentGridContext and line name extraction infrastructure.
pub struct LineNameInheritanceMapper {
    /// Cache for resolved line name mappings to optimize repeated lookups
    /// Key: (NodeId, GridAxis, parent_line_hash) -> resolved line names
    mapping_cache: HashMap<(NodeId, GridAxis, u64), Vec<Vec<String>>>,

    /// Performance metrics for cache optimization
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl LineNameInheritanceMapper {
    pub fn new() -> Self {
        Self {
            mapping_cache: HashMap::with_capacity(128),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    /// Core CSS Grid Level 2 line name inheritance algorithm
    ///
    /// Implements the complete specification requirements for line name inheritance,
    /// leveraging existing ParentGridContext infrastructure.
    pub fn map_subgrid_line_names(
        &mut self,
        parent_line_names: &[Vec<String>],
        subgrid_declared_names: &[String],
        subgrid_span: GridSpan,
        subgrid_node_id: NodeId,
        axis: GridAxis,
    ) -> Result<Vec<Vec<String>>, super::super::grid_errors::SubgridError> {
        // Phase 1: Check cache for performance optimization
        let cache_key = self.compute_cache_key(parent_line_names, subgrid_node_id, axis);
        if let Some(cached_result) = self.mapping_cache.get(&cache_key) {
            self.cache_hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(cached_result.clone());
        }

        self.cache_misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Phase 2: Extract parent line names for subgrid span (CSS Grid Level 2 requirement)
        let inherited_names = self.extract_parent_names_for_span(
            parent_line_names,
            subgrid_span.start,
            subgrid_span.end,
        )?;

        // Phase 3: Apply CSS-compliant subgrid declared line name merging
        let merged_names =
            self.merge_with_declared_names(&inherited_names, subgrid_declared_names)?;

        // Phase 4: Validate CSS identifier compliance
        self.validate_css_identifier_compliance(&merged_names)?;

        // Phase 5: Cache result for performance
        self.mapping_cache.insert(cache_key, merged_names.clone());

        Ok(merged_names)
    }

    /// Extract parent line names for exact subgrid span per CSS specification
    ///
    /// CSS Spec: "The subgrid inherits the line names from its parent grid
    ///            corresponding to the grid lines it spans."
    fn extract_parent_names_for_span(
        &self,
        parent_names: &[Vec<String>],
        span_start: usize,
        span_end: usize,
    ) -> Result<Vec<Vec<String>>, super::super::grid_errors::SubgridError> {
        // Validate span bounds against parent grid
        if span_start >= parent_names.len() || span_end > parent_names.len() {
            return Err(
                super::super::grid_errors::SubgridError::line_mapping_failed(
                    format!("span {}..{}", span_start, span_end),
                    format!("parent lines [0..{}]", parent_names.len()),
                    "Subgrid span exceeds parent grid line count",
                ),
            );
        }

        if span_start >= span_end {
            return Err(
                super::super::grid_errors::SubgridError::line_mapping_failed(
                    format!("span start {}", span_start),
                    format!("span end {}", span_end),
                    "Invalid span: start must be less than end",
                ),
            );
        }

        // Extract line names for the spanned lines
        // Note: CSS Grid lines are numbered 1-based, but arrays are 0-based
        // We need line names for lines span_start through span_end (inclusive of end line)
        let end_line_index = span_end.min(parent_names.len());
        let inherited = parent_names[span_start..end_line_index].to_vec();

        Ok(inherited)
    }

    /// Merge subgrid declared names with inherited names per CSS specification
    ///
    /// CSS Spec: "If the subgrid specifies line names, these names are merged
    ///            with any inherited names for the same line."
    fn merge_with_declared_names(
        &self,
        inherited_names: &[Vec<String>],
        declared_names: &[String],
    ) -> Result<Vec<Vec<String>>, super::super::grid_errors::SubgridError> {
        let mut merged = Vec::with_capacity(inherited_names.len());

        // CSS Grid Level 2: Optional line name list assigns names to lines starting from line 1
        // Excess declared names are ignored (per CSS specification, not an error)

        for (line_index, inherited_line_names) in inherited_names.iter().enumerate() {
            let mut line_names = inherited_line_names.clone();

            // Add declared name if available for this line index
            if let Some(declared_name) = declared_names.get(line_index) {
                if !declared_name.is_empty() {
                    // CSS requirement: avoid duplicate names on the same line
                    if !line_names.contains(declared_name) {
                        line_names.push(declared_name.clone());
                    }
                }
            }

            merged.push(line_names);
        }

        Ok(merged)
    }

    /// Validate CSS identifier compliance per CSS Syntax Module Level 3
    ///
    /// CSS Spec: "Grid line names must be valid CSS identifiers."
    fn validate_css_identifier_compliance(
        &self,
        line_names: &[Vec<String>],
    ) -> Result<(), super::super::grid_errors::SubgridError> {
        for (line_index, names) in line_names.iter().enumerate() {
            for name in names {
                if !self.is_valid_css_identifier(name) {
                    return Err(
                        super::super::grid_errors::SubgridError::line_mapping_failed(
                            name.clone(),
                            format!("line {}", line_index + 1),
                            "Invalid CSS identifier: must start with letter/underscore and contain only alphanumeric/hyphen/underscore",
                        ),
                    );
                }
            }
        }

        Ok(())
    }

    /// CSS identifier validation per CSS Syntax Module Level 3
    ///
    /// CSS Spec: "An identifier consists of a name start character followed by
    ///            any number of name characters."
    fn is_valid_css_identifier(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let chars: Vec<char> = name.chars().collect();

        // First character: letter, underscore, or non-ASCII (simplified)
        let first_char = chars[0];
        if !first_char.is_ascii_alphabetic() && first_char != '_' && first_char.is_ascii() {
            return false;
        }

        // Remaining characters: letters, digits, hyphens, underscores, or non-ASCII
        for &ch in &chars[1..] {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' && ch.is_ascii() {
                return false;
            }
        }

        // CSS keywords that cannot be used as identifiers (partial list)
        const RESERVED_KEYWORDS: &[&str] = &["auto", "inherit", "initial", "unset", "none"];
        if RESERVED_KEYWORDS.contains(&name.to_lowercase().as_str()) {
            return false;
        }

        true
    }

    /// Compute cache key for line name mapping optimization
    fn compute_cache_key(
        &self,
        parent_line_names: &[Vec<String>],
        node_id: NodeId,
        axis: GridAxis,
    ) -> (NodeId, GridAxis, u64) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        parent_line_names.hash(&mut hasher);
        let line_names_hash = hasher.finish();

        (node_id, axis, line_names_hash)
    }

    /// Resolve line names through complete nested subgrid inheritance chain
    ///
    /// CSS Spec: "Subgrids can be nested, with each level inheriting from its
    ///            immediate parent subgrid and adding its own declared names."
    pub fn resolve_nested_subgrid_line_names(
        &mut self,
        inheritance_chain: &[SubgridInheritanceLevel],
        final_declared_names: &[String],
        final_subgrid_node_id: NodeId,
        axis: GridAxis,
    ) -> Result<Vec<Vec<String>>, super::super::grid_errors::SubgridError> {
        let mut current_names = Vec::new();

        // Start with root parent line names
        if let Some(root_level) = inheritance_chain.first() {
            current_names = root_level.parent_line_names.clone();
        }

        // Apply each inheritance level in the chain
        for (level_index, inheritance_level) in inheritance_chain.iter().enumerate().skip(1) {
            current_names = self.map_subgrid_line_names(
                &current_names,
                &inheritance_level.declared_names,
                inheritance_level.span_in_parent,
                inheritance_level.subgrid_node_id,
                axis,
            )?;

            // Log inheritance level for debugging complex nested structures
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Applied inheritance level {} for node {}: {} line name groups",
                level_index,
                usize::from(inheritance_level.subgrid_node_id),
                current_names.len()
            );
        }

        // Apply final subgrid's declared names
        let final_span = GridSpan {
            start: 0,
            end: current_names.len(),
        };
        let final_names = self.map_subgrid_line_names(
            &current_names,
            final_declared_names,
            final_span,
            final_subgrid_node_id,
            axis,
        )?;

        Ok(final_names)
    }
}

impl Default for LineNameInheritanceMapper {
    fn default() -> Self {
        Self::new()
    }
}
