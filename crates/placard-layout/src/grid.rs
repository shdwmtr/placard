use placard_font::FontSet;
use placard_html::{Dom, NodeId};
use placard_style::{BoxSizing, ComputedStyle, Dimension, TrackSize};

use crate::block::{layout_block, measure_intrinsic_width};
use crate::flex::is_in_flow_child;
use crate::tree::{LayoutNodeId, LayoutTree};

struct Cell {
    node: NodeId,
    row: usize,
    col: usize,
}

/// Resolves column tracks against `content_width`: `px`/`%` tracks first,
/// then `auto` tracks sized to the widest cell assigned to them (via
/// [`measure_intrinsic_width`]), then whatever's left over split among
/// `fr` tracks by weight -- the same free-space-after-fixed-tracks model
/// `fr` uses in real CSS Grid.
fn resolve_column_widths(
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    columns: &[TrackSize],
    cells: &[Cell],
    content_width: f32,
    column_gap: f32,
) -> Vec<f32> {
    let num_cols = columns.len();
    let mut widths = vec![0.0f32; num_cols];
    let mut fr = vec![0.0f32; num_cols];
    let mut total_fixed = 0.0f32;

    for (c, track) in columns.iter().enumerate() {
        match track {
            TrackSize::Px(v) => {
                widths[c] = *v;
                total_fixed += *v;
            }
            TrackSize::Percent(p) => {
                let w = (content_width * p / 100.0).max(0.0);
                widths[c] = w;
                total_fixed += w;
            }
            TrackSize::Fr(f) => fr[c] = *f,
            TrackSize::Auto => {
                let w = cells
                    .iter()
                    .filter(|cell| cell.col == c)
                    .map(|cell| measure_intrinsic_width(dom, styles, fonts, cell.node))
                    .fold(0.0f32, f32::max);
                widths[c] = w;
                total_fixed += w;
            }
        }
    }

    let total_fr: f32 = fr.iter().sum();
    if total_fr > 0.0 {
        let total_gap = column_gap * num_cols.saturating_sub(1) as f32;
        let remaining = (content_width - total_fixed - total_gap).max(0.0);
        for c in 0..num_cols {
            if fr[c] > 0.0 {
                widths[c] = remaining * (fr[c] / total_fr);
            }
        }
    }

    widths
}

/// Mirrors `layout_block`'s own explicit-height override, used to know
/// whether `fr` row tracks have a definite total height to divide up (see
/// `resolve_row_heights`).
fn explicit_content_height(container_style: &ComputedStyle) -> Option<f32> {
    if let Dimension::Px(v) = container_style.height? {
        Some(match container_style.box_sizing {
            BoxSizing::BorderBox => (v
                - container_style.padding[0]
                - container_style.padding[2]
                - container_style.border_width[0]
                - container_style.border_width[2])
                .max(0.0),
            BoxSizing::ContentBox => v,
        })
    } else {
        None
    }
}

/// Resolves row heights. Unlike columns, a row's natural height needs no
/// special shrink-to-fit measurement -- height in this engine is always
/// content-driven already, so a plain scratch `layout_block` call at the
/// cell's resolved column width gives its natural height directly.
///
/// `fr` rows only get a share of *definite* leftover space, which only
/// exists when the grid container has an explicit `height`; otherwise
/// (matching the equivalent rule for `flex-direction: column`) they fall
/// back to their natural content size, same as `auto`.
fn resolve_row_heights(
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    container_style: &ComputedStyle,
    rows: &[TrackSize],
    num_rows: usize,
    cells: &[Cell],
    col_widths: &[f32],
    row_gap: f32,
) -> Vec<f32> {
    let row_track = |r: usize| rows.get(r).copied().unwrap_or(TrackSize::Auto);

    let natural: Vec<f32> = (0..num_rows)
        .map(|r| {
            cells
                .iter()
                .filter(|cell| cell.row == r)
                .map(|cell| {
                    let mut scratch = LayoutTree::empty();
                    let id = layout_block(
                        &mut scratch,
                        dom,
                        styles,
                        fonts,
                        cell.node,
                        0.0,
                        0.0,
                        col_widths[cell.col],
                    );
                    scratch.get(id).rect.height
                })
                .fold(0.0f32, f32::max)
        })
        .collect();

    let explicit_height = explicit_content_height(container_style);
    let mut heights = vec![0.0f32; num_rows];
    let mut is_fr = vec![false; num_rows];
    let mut fr_weight = vec![0.0f32; num_rows];
    let mut total_fixed = 0.0f32;

    for r in 0..num_rows {
        match row_track(r) {
            TrackSize::Px(v) => {
                heights[r] = v;
                total_fixed += v;
            }
            TrackSize::Percent(p) => {
                let h = explicit_height
                    .map(|eh| eh * p / 100.0)
                    .unwrap_or(natural[r]);
                heights[r] = h;
                total_fixed += h;
            }
            TrackSize::Fr(f) => {
                if explicit_height.is_some() {
                    is_fr[r] = true;
                    fr_weight[r] = f;
                } else {
                    heights[r] = natural[r];
                    total_fixed += natural[r];
                }
            }
            TrackSize::Auto => {
                heights[r] = natural[r];
                total_fixed += natural[r];
            }
        }
    }

    let total_fr: f32 = fr_weight.iter().sum();
    if total_fr > 0.0
        && let Some(eh) = explicit_height
    {
        let total_gap = row_gap * num_rows.saturating_sub(1) as f32;
        let remaining = (eh - total_fixed - total_gap).max(0.0);
        for r in 0..num_rows {
            if is_fr[r] {
                heights[r] = remaining * (fr_weight[r] / total_fr);
            }
        }
    }

    heights
}

pub(crate) fn layout_grid(
    tree: &mut LayoutTree,
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    node: NodeId,
    container_style: &ComputedStyle,
    content_x: f32,
    content_start_y: f32,
    content_width: f32,
) -> (Vec<LayoutNodeId>, f32) {
    let children: Vec<NodeId> = dom
        .children(node)
        .filter(|&child| is_in_flow_child(dom, styles, child))
        .collect();

    if children.is_empty() {
        return (Vec::new(), 0.0);
    }

    let columns = if container_style.grid_template_columns.is_empty() {
        vec![TrackSize::Auto]
    } else {
        container_style.grid_template_columns.clone()
    };
    let num_cols = columns.len();

    let column_gap = container_style.column_gap;
    let row_gap = container_style.row_gap;

    let cells: Vec<Cell> = children
        .into_iter()
        .enumerate()
        .map(|(i, node)| Cell {
            node,
            row: i / num_cols,
            col: i % num_cols,
        })
        .collect();
    let num_rows = cells.last().map_or(0, |c| c.row + 1);

    let col_widths = resolve_column_widths(
        dom,
        styles,
        fonts,
        &columns,
        &cells,
        content_width,
        column_gap,
    );
    let mut col_x = vec![0.0f32; num_cols];
    {
        let mut cursor = content_x;
        for c in 0..num_cols {
            col_x[c] = cursor;
            cursor += col_widths[c] + column_gap;
        }
    }

    let row_heights = resolve_row_heights(
        dom,
        styles,
        fonts,
        container_style,
        &container_style.grid_template_rows,
        num_rows,
        &cells,
        &col_widths,
        row_gap,
    );
    let mut row_y = vec![0.0f32; num_rows];
    {
        let mut cursor = content_start_y;
        for r in 0..num_rows {
            row_y[r] = cursor;
            cursor += row_heights[r] + row_gap;
        }
    }

    let mut result_ids = Vec::with_capacity(cells.len());
    for cell in &cells {
        let id = layout_block(
            tree,
            dom,
            styles,
            fonts,
            cell.node,
            col_x[cell.col],
            row_y[cell.row],
            col_widths[cell.col],
        );
        // Grid items stretch to fill their cell by default; this engine has
        // no way to re-flow a box's children against a new height, so (as
        // with flex's `align-items: stretch`) only the box itself grows.
        tree.set_height(id, row_heights[cell.row]);
        result_ids.push(id);
    }

    let content_height = if num_rows == 0 {
        0.0
    } else {
        row_y[num_rows - 1] + row_heights[num_rows - 1] - content_start_y
    };

    (result_ids, content_height)
}
