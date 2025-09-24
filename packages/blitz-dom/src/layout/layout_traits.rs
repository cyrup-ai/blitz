//! Layout trait implementations for BaseDocument
//!
//! This module contains all the taffy trait implementations that enable
//! BaseDocument to participate in layout algorithms (block, flexbox, grid).

use std::cell::Ref;

use markup5ever::local_name;
use taffy::{
    CollapsibleMarginSet, FlexDirection, LayoutPartialTree, NodeId, ResolveOrZero, RoundTree,
    Style, TraversePartialTree, TraverseTree, compute_block_layout, compute_cached_layout,
    compute_flexbox_layout, compute_leaf_layout, prelude::*,
};

use super::grid_preprocessing::preprocess_and_compute_grid_layout;
use super::replaced::{ReplacedContext, replaced_measure_function};
use super::resolve_calc_value;
use super::table::TableTreeWrapper;
use super::tree_iteration::RefCellChildIter;
use crate::{
    BaseDocument,
    node::{ImageData, NodeData, SpecialElementData},
};

impl BaseDocument {
    pub(crate) fn node_from_id(&self, node_id: taffy::prelude::NodeId) -> &crate::node::Node {
        &self.nodes[node_id.into()]
    }
    pub(crate) fn node_from_id_mut(
        &mut self,
        node_id: taffy::prelude::NodeId,
    ) -> &mut crate::node::Node {
        &mut self.nodes[node_id.into()]
    }

    /// Resolve parent grid context using optimized cached algorithms
    ///
    /// This method provides efficient O(log n) parent grid context resolution
    /// with caching support, replacing the previous O(n²) approach.
    pub fn resolve_parent_grid_context_cached(
        &self,
        node_id: NodeId,
    ) -> Result<Option<super::grid_context::ParentGridContext>, super::grid_context::GridContextError>
    {
        super::grid_context::resolve_parent_grid_context_for_generic_tree_efficient(self, node_id)
    }
}

impl TraversePartialTree for BaseDocument {
    type ChildIter<'a> = RefCellChildIter<'a>;

    fn child_ids(&self, node_id: NodeId) -> Self::ChildIter<'_> {
        let layout_children = self.node_from_id(node_id).layout_children.borrow();
        RefCellChildIter::new(Ref::map(layout_children, |children| {
            match children.as_ref() {
                Some(c) => c.as_slice(),
                None => &[],
            }
        }))
    }

    fn child_count(&self, node_id: NodeId) -> usize {
        self.node_from_id(node_id)
            .layout_children
            .borrow()
            .as_ref()
            .map(|c| c.len())
            .unwrap_or(0)
    }

    fn get_child_id(&self, node_id: NodeId, index: usize) -> NodeId {
        let layout_children = self.node_from_id(node_id).layout_children.borrow();
        match layout_children.as_ref() {
            Some(children) if index < children.len() => NodeId::from(children[index]),
            Some(_) => {
                eprintln!(
                    "Warning: Child index {} out of bounds for node {:?}",
                    index, node_id
                );
                node_id // Return the parent node as fallback
            }
            None => {
                eprintln!(
                    "Warning: Node {:?} has no layout children but child at index {} was requested",
                    node_id, index
                );
                node_id // Return the parent node as fallback
            }
        }
    }
}

impl TraverseTree for BaseDocument {}

impl LayoutPartialTree for BaseDocument {
    type CoreContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: NodeId) -> &Style {
        // On-demand conversion for memory-constrained scenarios
        // Uses smart invalidation to only recompute when stylo styles change
        match self.get_or_compute_taffy_style(node_id) {
            Ok(style) => style,
            Err(_) => {
                // Return node's existing style as fallback - always valid
                self.node_from_id(node_id).style()
            }
        }
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.node_from_id_mut(node_id).unrounded_layout = *layout;
    }

    fn resolve_calc_value(&self, calc_ptr: *const (), parent_size: f32) -> f32 {
        resolve_calc_value(calc_ptr, parent_size)
    }

    fn compute_child_layout(
        &mut self,
        node_id: NodeId,
        inputs: taffy::tree::LayoutInput,
    ) -> taffy::tree::LayoutOutput {
        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let node = &mut tree.nodes[node_id.into()];

            let font_styles = node.primary_styles().map(|style| {
                use style::values::computed::font::LineHeight;

                let font_size = style.clone_font_size().used_size().px();
                let line_height = match style.clone_line_height() {
                    LineHeight::Normal => font_size * 1.2,
                    LineHeight::Number(num) => font_size * num.0,
                    LineHeight::Length(value) => value.0.px(),
                    // Note: MozBlockHeight variant only available with gecko feature
                };

                (font_size, line_height)
            });
            let font_size = font_styles.map(|s| s.0);
            let resolved_line_height = font_styles.map(|s| s.1);

            match &mut node.data {
                NodeData::Text(data) => {
                    // With the new "inline context" architecture all text nodes should be wrapped in an "inline layout context"
                    // and should therefore never be measured individually.
                    println!(
                        "ERROR: Tried to lay out text node individually ({})",
                        usize::from(node_id)
                    );
                    dbg!(data);
                    taffy::LayoutOutput::HIDDEN
                }
                NodeData::Element(element_data) | NodeData::AnonymousBlock(element_data) => {
                    // TODO: deduplicate with single-line text input
                    if *element_data.name.local == *"textarea" {
                        let rows = element_data
                            .attr(local_name!("rows"))
                            .and_then(|val| val.parse::<f32>().ok())
                            .unwrap_or(2.0);

                        let cols = element_data
                            .attr(local_name!("cols"))
                            .and_then(|val| val.parse::<f32>().ok());

                        return compute_leaf_layout(
                            inputs,
                            node.style(),
                            resolve_calc_value,
                            |_known_size, _available_space| taffy::Size {
                                width: cols
                                    .map(|cols| cols * font_size.unwrap_or(16.0) * 0.6)
                                    .unwrap_or(300.0),
                                height: resolved_line_height.unwrap_or(16.0) * rows,
                            },
                        );
                    }

                    if *element_data.name.local == *"input" {
                        match element_data.attr(local_name!("type")) {
                            // if the input type is hidden, hide it
                            Some("hidden") => {
                                node.style_mut().display = Display::None;
                                return taffy::LayoutOutput::HIDDEN;
                            }
                            Some("checkbox") => {
                                return compute_leaf_layout(
                                    inputs,
                                    &node.style(),
                                    resolve_calc_value,
                                    |_known_size, _available_space| {
                                        let width = node.style().size.width.resolve_or_zero(
                                            inputs.parent_size.width,
                                            resolve_calc_value,
                                        );
                                        let height = node.style().size.height.resolve_or_zero(
                                            inputs.parent_size.height,
                                            resolve_calc_value,
                                        );
                                        let min_size = width.min(height);
                                        taffy::Size {
                                            width: min_size,
                                            height: min_size,
                                        }
                                    },
                                );
                            }
                            None | Some("text" | "password" | "email") => {
                                return compute_leaf_layout(
                                    inputs,
                                    &node.style(),
                                    resolve_calc_value,
                                    |_known_size, _available_space| taffy::Size {
                                        width: 300.0,
                                        height: resolved_line_height.unwrap_or(16.0),
                                    },
                                );
                            }
                            _ => {}
                        }
                    }

                    if *element_data.name.local == *"img"
                        || *element_data.name.local == *"canvas"
                        || (cfg!(feature = "svg") && *element_data.name.local == *"svg")
                    {
                        // Get width and height attributes on image element
                        let attr_size = taffy::Size {
                            width: element_data
                                .attr(local_name!("width"))
                                .and_then(|val| val.parse::<f32>().ok()),
                            height: element_data
                                .attr(local_name!("height"))
                                .and_then(|val| val.parse::<f32>().ok()),
                        };

                        // Get image's native size
                        let inherent_size = match &element_data.special_data {
                            SpecialElementData::Image(image_data) => match &**image_data {
                                ImageData::Raster(image) => taffy::Size {
                                    width: image.width as f32,
                                    height: image.height as f32,
                                },
                                #[cfg(feature = "svg")]
                                ImageData::Svg(svg) => {
                                    let size = svg.size();
                                    taffy::Size {
                                        width: size.width(),
                                        height: size.height(),
                                    }
                                }
                                ImageData::None => taffy::Size::ZERO,
                            },
                            SpecialElementData::Canvas(_) => taffy::Size {
                                width: 300.0,  // HTML5 canvas default width
                                height: 150.0, // HTML5 canvas default height
                            },
                            SpecialElementData::None => taffy::Size::ZERO,
                            _ => unreachable!(),
                        };

                        let replaced_context = ReplacedContext {
                            inherent_size,
                            attr_size,
                        };

                        let computed = replaced_measure_function(
                            inputs.known_dimensions,
                            inputs.parent_size,
                            &replaced_context,
                            &node.style(),
                            false,
                        );

                        return taffy::LayoutOutput {
                            size: computed,
                            content_size: computed,
                            first_baselines: taffy::Point::NONE,
                            top_margin: CollapsibleMarginSet::ZERO,
                            bottom_margin: CollapsibleMarginSet::ZERO,
                            margins_can_collapse_through: false,
                        };
                    }

                    if node.flags.is_table_root() {
                        // Build table context on-demand with proper preprocessing
                        let (table_context, layout_children) = super::table::build_table_context(
                            tree, 
                            usize::from(node_id)
                        );
                        
                        // Update the node's layout children to use the computed table layout
                        let table_node = &mut tree.nodes[usize::from(node_id)];
                        *table_node.layout_children.borrow_mut() = Some(layout_children);
                        
                        // Create table wrapper with proper context
                        let context = std::sync::Arc::new(table_context);
                        let mut table_wrapper = TableTreeWrapper {
                            doc: tree,
                            ctx: context,
                        };
                        
                        // Compute proper CSS table layout using grid engine
                        return taffy::compute_grid_layout(&mut table_wrapper, node_id, inputs);
                    }

                    if node.flags.is_inline_root() {
                        return tree.compute_inline_layout(usize::from(node_id), inputs);
                    }

                    // The default CSS file will set
                    match node.style().display {
                        Display::Block => compute_block_layout(tree, node_id, inputs),
                        Display::Flex => compute_flexbox_layout(tree, node_id, inputs),
                        Display::Grid => preprocess_and_compute_grid_layout(tree, node_id, inputs),
                        Display::None => taffy::LayoutOutput::HIDDEN,
                    }
                }
                NodeData::Document => compute_block_layout(tree, node_id, inputs),

                _ => taffy::LayoutOutput::HIDDEN,
            }
        })
    }
}

impl taffy::CacheTree for BaseDocument {
    #[inline]
    fn cache_get(
        &self,
        node_id: NodeId,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        run_mode: taffy::RunMode,
    ) -> Option<taffy::LayoutOutput> {
        self.node_from_id(node_id)
            .cache
            .get(known_dimensions, available_space, run_mode)
    }

    #[inline]
    fn cache_store(
        &mut self,
        node_id: NodeId,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        run_mode: taffy::RunMode,
        layout_output: taffy::LayoutOutput,
    ) {
        self.node_from_id_mut(node_id).cache.store(
            known_dimensions,
            available_space,
            run_mode,
            layout_output,
        );
    }

    #[inline]
    fn cache_clear(&mut self, node_id: NodeId) {
        self.node_from_id_mut(node_id).cache.clear();
    }
}

impl taffy::LayoutBlockContainer for BaseDocument {
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl taffy::LayoutFlexboxContainer for BaseDocument {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl taffy::LayoutGridContainer for BaseDocument {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: NodeId) -> Self::GridContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: NodeId) -> Self::GridItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl RoundTree for BaseDocument {
    fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
        self.node_from_id(node_id).unrounded_layout
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.node_from_id_mut(node_id).final_layout = *layout;
    }
}

impl PrintTree for BaseDocument {
    fn get_debug_label(&self, node_id: NodeId) -> &'static str {
        let node = &self.node_from_id(node_id);
        let style = &node.style();

        match node.data {
            NodeData::Document => "DOCUMENT",
            NodeData::Text { .. } => node.node_debug_str().leak(),
            NodeData::Comment => "COMMENT",
            NodeData::AnonymousBlock(_) => "ANONYMOUS BLOCK",
            NodeData::Element(_) => {
                let display = match style.display {
                    Display::Flex => match style.flex_direction {
                        FlexDirection::Row | FlexDirection::RowReverse => "FLEX ROW",
                        FlexDirection::Column | FlexDirection::ColumnReverse => "FLEX COL",
                    },
                    Display::Grid => "GRID",
                    Display::Block => "BLOCK",
                    Display::None => "NONE",
                };
                format!("{} ({})", node.node_debug_str(), display).leak()
            }
        }
    }

    fn get_final_layout(&self, node_id: NodeId) -> Layout {
        self.node_from_id(node_id).final_layout
    }
}
