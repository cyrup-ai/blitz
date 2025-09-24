use std::{ops::Range, sync::Arc};

use markup5ever::local_name;
use style::computed_values::table_layout::T as TableLayout;
use style::values::specified::box_::{DisplayInside, DisplayOutside};
use taffy::{
    Dimension, LayoutPartialTree as _, Line, MaxTrackSizingFunction, MinTrackSizingFunction,
    ResolveOrZero, compute_leaf_layout, style_helpers,
};

use super::resolve_calc_value;
use crate::BaseDocument;

pub struct TableTreeWrapper<'doc> {
    pub(crate) doc: &'doc mut BaseDocument,
    pub(crate) ctx: Arc<TableContext>,
}

#[derive(Debug, Clone)]
pub struct TableContext {
    style: taffy::Style,
    items: Vec<TableItem>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TableItemKind {
    Row,
    Cell,
}

#[derive(Debug, Clone)]
pub struct TableItem {
    kind: TableItemKind,
    node_id: usize,
    style: taffy::Style,
}

pub(crate) fn build_table_context(
    doc: &mut BaseDocument,
    table_root_node_id: usize,
) -> (TableContext, Vec<usize>) {
    let mut items: Vec<TableItem> = Vec::new();
    let mut row = 0u16;
    let mut col = 0u16;

    let root_node = &mut doc.nodes[table_root_node_id];

    let children = std::mem::take(&mut root_node.children);

    let Some(stylo_styles) = root_node.primary_styles() else {
        panic!("Ignoring table because it has no styles");
    };

    // Use production-quality scrollbar width detection
    let device = doc.stylist.device();
    let mut style = stylo_taffy::to_taffy_style_with_device(&stylo_styles, &device);
    style.grid_auto_columns = Vec::new();
    style.grid_auto_rows = Vec::new();

    let is_fixed = match stylo_styles.clone_table_layout() {
        TableLayout::Fixed => true,
        TableLayout::Auto => false,
    };

    drop(stylo_styles);

    let mut column_sizes: Vec<taffy::Dimension> = Vec::new();
    for child_id in children.iter().copied() {
        collect_table_cells(
            doc,
            child_id,
            is_fixed,
            &mut row,
            &mut col,
            &mut items,
            &mut column_sizes,
        );
    }
    column_sizes.resize(col as usize, Dimension::auto());

    style.grid_template_columns = column_sizes
        .into_iter()
        .map(|_| {
            taffy::MinMax {
                min: MinTrackSizingFunction::auto(),
                max: MaxTrackSizingFunction::auto(),
            }
            .into()
        })
        .collect();
    style.grid_template_rows = vec![
        taffy::MinMax {
            min: MinTrackSizingFunction::auto(),
            max: MaxTrackSizingFunction::auto(),
        }
        .into();
        row as usize
    ];

    let layout_children = items
        .iter()
        .filter(|item| item.kind == TableItemKind::Cell)
        .map(|cell| cell.node_id)
        .collect();
    let root_node = &mut doc.nodes[table_root_node_id];
    root_node.children = children;

    (TableContext { style, items }, layout_children)
}

pub(crate) fn collect_table_cells(
    doc: &mut BaseDocument,
    node_id: usize,
    is_fixed: bool,
    row: &mut u16,
    col: &mut u16,
    cells: &mut Vec<TableItem>,
    columns: &mut Vec<Dimension>,
) {
    let node = &doc.nodes[node_id];

    if !node.is_element() {
        return;
    }

    let Some(display) = node.primary_styles().map(|s| s.clone_display()) else {
        println!("Ignoring table descendent because it has no styles");
        return;
    };

    if display.outside() == DisplayOutside::None {
        return;
    }

    match display.inside() {
        DisplayInside::TableRowGroup
        | DisplayInside::TableHeaderGroup
        | DisplayInside::TableFooterGroup
        | DisplayInside::Contents => {
            let children = std::mem::take(&mut doc.nodes[node_id].children);
            for child_id in children.iter().copied() {
                collect_table_cells(doc, child_id, is_fixed, row, col, cells, columns);
            }
            doc.nodes[node_id].children = children;
        }
        DisplayInside::TableRow => {
            *row += 1;
            *col = 0;

            {
                let stylo_style = match node.primary_styles() {
                    Some(styles) => styles,
                    None => {
                        eprintln!(
                            "Warning: Table row node {} has no primary styles, skipping",
                            node_id
                        );
                        return;
                    }
                };
                let device = doc.stylist.device();
                let mut style = stylo_taffy::to_taffy_style_with_device(&*stylo_style, &device);
                style.grid_column = Line {
                    start: style_helpers::line(1),
                    end: style_helpers::line(-1),
                };
                style.grid_row = Line {
                    start: style_helpers::line(*row as i16),
                    end: taffy::GridPlacement::Span(1),
                };
                cells.push(TableItem {
                    kind: TableItemKind::Row,
                    node_id,
                    style,
                });
            }

            let children = std::mem::take(&mut doc.nodes[node_id].children);
            for child_id in children.iter().copied() {
                collect_table_cells(doc, child_id, is_fixed, row, col, cells, columns);
            }
            doc.nodes[node_id].children = children;
        }
        DisplayInside::TableCell => {
            let stylo_style = match node.primary_styles() {
                Some(styles) => styles,
                None => {
                    eprintln!(
                        "Warning: Table cell node {} has no primary styles, skipping",
                        node_id
                    );
                    return;
                }
            };
            let colspan: u16 = node
                .attr(local_name!("colspan"))
                .and_then(|val| val.parse().ok())
                .unwrap_or(1);
            let device = doc.stylist.device();
            let mut style = stylo_taffy::to_taffy_style_with_device(&*stylo_style, &device);

            // TODO: account for padding/border/margin
            if *row == 1 {
                let column = match style.size.width.tag() {
                    taffy::CompactLength::LENGTH_TAG => {
                        let len = style.size.width.value();
                        let padding = style
                            .padding
                            .resolve_or_zero(None::<f32>, |calc_ptr, parent_size| {
                                resolve_calc_value(calc_ptr, parent_size)
                            });
                        Dimension::length(len + padding.left + padding.right)
                    }
                    taffy::CompactLength::PERCENT_TAG => {
                        Dimension::percent(style.size.width.value())
                    }
                    taffy::CompactLength::AUTO_TAG => Dimension::auto(),
                    // Fallback to auto for unknown dimension types instead of panicking
                    _ => Dimension::auto(),
                };
                columns.push(column);
            } else if !is_fixed
                && (*col as usize) < columns.len()
                && taffy::CompactLength::LENGTH_TAG == style.size.width.tag()
            {
                let new_len = style.size.width.value();
                let tag = columns[*col as usize].tag();
                let value = columns[*col as usize].value();
                columns[*col as usize] = match tag {
                    taffy::CompactLength::LENGTH_TAG => Dimension::length(value.max(new_len)),
                    taffy::CompactLength::AUTO_TAG => Dimension::length(new_len),
                    taffy::CompactLength::PERCENT_TAG => Dimension::percent(value),
                    // Fallback to auto for unknown dimension types instead of panicking
                    _ => Dimension::auto(),
                }
            }

            style.grid_column = Line {
                start: style_helpers::line((*col + 1) as i16),
                end: taffy::GridPlacement::Span(colspan),
            };
            style.grid_row = Line {
                start: style_helpers::line(*row as i16),
                end: taffy::GridPlacement::Span(1),
            };
            style.size.width = Dimension::auto();
            cells.push(TableItem {
                kind: TableItemKind::Cell,
                node_id,
                style,
            });

            *col += colspan;
        }
        DisplayInside::None => {
            // Ignore
        }
        _ => {
            println!(
                "Warning: ignoring non-table typed descendent of table ({:?})",
                display.inside()
            );
        }
    }
}

pub struct RangeIter(Range<usize>);

impl Iterator for RangeIter {
    type Item = taffy::NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(taffy::NodeId::from)
    }
}

impl taffy::TraversePartialTree for TableTreeWrapper<'_> {
    type ChildIter<'a>
        = RangeIter
    where
        Self: 'a;

    #[inline(always)]
    fn child_ids(&self, _node_id: taffy::NodeId) -> Self::ChildIter<'_> {
        RangeIter(0..self.ctx.items.len())
    }

    #[inline(always)]
    fn child_count(&self, node_id: taffy::NodeId) -> usize {
        self.doc.child_count(node_id)
    }

    #[inline(always)]
    fn get_child_id(&self, _node_id: taffy::NodeId, index: usize) -> taffy::NodeId {
        index.into()
    }
}
impl taffy::TraverseTree for TableTreeWrapper<'_> {}

impl taffy::LayoutPartialTree for TableTreeWrapper<'_> {
    type CoreContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type CustomIdent = String;

    fn get_core_container_style(&self, _node_id: taffy::NodeId) -> &taffy::Style {
        &self.ctx.style
    }

    fn resolve_calc_value(&self, calc_ptr: *const (), parent_size: f32) -> f32 {
        resolve_calc_value(calc_ptr, parent_size)
    }

    fn set_unrounded_layout(&mut self, node_id: taffy::NodeId, layout: &taffy::Layout) {
        let node_id = taffy::NodeId::from(self.ctx.items[usize::from(node_id)].node_id);
        self.doc.set_unrounded_layout(node_id, layout)
    }

    fn compute_child_layout(
        &mut self,
        node_id: taffy::NodeId,
        inputs: taffy::tree::LayoutInput,
    ) -> taffy::LayoutOutput {
        let cell = &self.ctx.items[usize::from(node_id)];
        match cell.kind {
            TableItemKind::Row => {
                compute_leaf_layout(inputs, &cell.style, resolve_calc_value, |_, _| {
                    taffy::Size::ZERO
                })
            }
            TableItemKind::Cell => {
                let node_id = taffy::NodeId::from(cell.node_id);
                self.doc.compute_child_layout(node_id, inputs)
            }
        }
    }
}

impl taffy::LayoutGridContainer for TableTreeWrapper<'_> {
    type GridContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    type GridItemStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: taffy::NodeId) -> Self::GridContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: taffy::NodeId) -> Self::GridItemStyle<'_> {
        &self.ctx.items[usize::from(child_node_id)].style
    }
}
