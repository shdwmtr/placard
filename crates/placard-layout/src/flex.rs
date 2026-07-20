use placard_font::FontSet;
use placard_html::{Dom, NodeId};
use placard_style::{
    AlignItems, BoxSizing, ComputedStyle, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent,
};

use crate::block::{is_out_of_flow, layout_block, measure_intrinsic_width};
use crate::tree::{LayoutNodeId, LayoutTree};

pub(crate) fn is_in_flow_child(dom: &Dom, styles: &[ComputedStyle], child: NodeId) -> bool {
    dom.tag(child).is_some()
        && styles[child.index()].display != Display::None
        && !is_out_of_flow(&styles[child.index()])
}

fn resolve_border_box_from_dimension(cs: &ComputedStyle, dim: Dimension, basis: f32) -> f32 {
    let specified = dim.resolve(basis);
    let padding_h = cs.padding[1] + cs.padding[3];
    let border_h = cs.border_width[1] + cs.border_width[3];
    match cs.box_sizing {
        BoxSizing::BorderBox => specified.max(0.0),
        BoxSizing::ContentBox => (specified + padding_h + border_h).max(0.0),
    }
}

/// The border-box main size a row-direction flex item starts from, before
/// grow/shrink are applied: an explicit `flex-basis`, else an explicit
/// `width`, else its measured shrink-to-fit natural width.
fn resolve_row_basis(
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    node: NodeId,
    content_width: f32,
) -> f32 {
    let cs = &styles[node.index()];
    if let Some(dim) = cs.flex_basis {
        resolve_border_box_from_dimension(cs, dim, content_width)
    } else if let Some(dim) = cs.width {
        resolve_border_box_from_dimension(cs, dim, content_width)
    } else {
        measure_intrinsic_width(dom, styles, fonts, node)
    }
}

/// Splits `remaining` free space (already reduced to zero by grow/shrink
/// when there's any weight to distribute it against) into a leading offset
/// before the first item and a fixed spacing added after every item,
/// implementing `justify-content`'s six keywords in one place for both
/// flex directions.
fn justify_offsets(justify: JustifyContent, remaining: f32, n: usize, gap: f32) -> (f32, f32) {
    if n <= 1 {
        return match justify {
            JustifyContent::FlexEnd => (remaining, gap),
            JustifyContent::Center => (remaining / 2.0, gap),
            _ => (0.0, gap),
        };
    }
    match justify {
        JustifyContent::FlexStart => (0.0, gap),
        JustifyContent::FlexEnd => (remaining, gap),
        JustifyContent::Center => (remaining / 2.0, gap),
        JustifyContent::SpaceBetween => (0.0, gap + remaining / (n - 1) as f32),
        JustifyContent::SpaceAround => {
            let share = remaining / n as f32;
            (share / 2.0, gap + share)
        }
        JustifyContent::SpaceEvenly => {
            let share = remaining / (n + 1) as f32;
            (share, gap + share)
        }
    }
}

struct RowItem {
    node: NodeId,
    margin: [f32; 4],
    basis: f32,
    grow: f32,
    shrink: f32,
    align: AlignItems,
    has_explicit_height: bool,
}

fn layout_flex_row(
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
    let items: Vec<RowItem> = dom
        .children(node)
        .filter(|&child| is_in_flow_child(dom, styles, child))
        .map(|child| {
            let cs = &styles[child.index()];
            RowItem {
                node: child,
                margin: [
                    cs.margin[0].unwrap_or(0.0),
                    cs.margin[1].unwrap_or(0.0),
                    cs.margin[2].unwrap_or(0.0),
                    cs.margin[3].unwrap_or(0.0),
                ],
                basis: resolve_row_basis(dom, styles, fonts, child, content_width),
                grow: cs.flex_grow,
                shrink: cs.flex_shrink,
                align: cs.align_self.unwrap_or(container_style.align_items),
                has_explicit_height: cs.height.is_some(),
            }
        })
        .collect();

    if items.is_empty() {
        return (Vec::new(), 0.0);
    }

    // Mirrors `layout_block`'s own explicit-height override (which runs
    // *after* this function returns, against whatever content_height it
    // gets back). Without this, a single-line row with an explicit height
    // would align its items against their natural (pre-override) cross
    // size instead of the container's real final height -- e.g.
    // `align-items: center` centering against the wrong band and leaving
    // the rest of the container's height empty below it.
    let explicit_content_height = if let Some(Dimension::Px(v)) = container_style.height {
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
    };

    let main_gap = container_style.column_gap;
    let cross_gap = container_style.row_gap;

    let mut lines: Vec<Vec<usize>> = Vec::new();
    if container_style.flex_wrap == FlexWrap::Wrap {
        let mut current = Vec::new();
        let mut used = 0.0f32;
        for (i, item) in items.iter().enumerate() {
            let outer = item.basis + item.margin[1] + item.margin[3];
            let needed = if current.is_empty() {
                outer
            } else {
                used + main_gap + outer
            };
            if !current.is_empty() && needed > content_width {
                lines.push(std::mem::take(&mut current));
                used = outer;
                current.push(i);
            } else {
                used = needed;
                current.push(i);
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    } else {
        lines.push((0..items.len()).collect());
    }

    let mut result_ids = Vec::new();
    let mut cursor_y = content_start_y;

    for (line_idx, line) in lines.iter().enumerate() {
        let outer_basis: Vec<f32> = line
            .iter()
            .map(|&i| items[i].basis + items[i].margin[1] + items[i].margin[3])
            .collect();
        let total_basis: f32 =
            outer_basis.iter().sum::<f32>() + main_gap * (line.len().saturating_sub(1)) as f32;
        let free = content_width - total_basis;

        let mut target_outer = outer_basis.clone();
        if free > 0.0 {
            let total_grow: f32 = line.iter().map(|&i| items[i].grow).sum();
            if total_grow > 0.0 {
                for (k, &i) in line.iter().enumerate() {
                    target_outer[k] += free * (items[i].grow / total_grow);
                }
            }
        } else if free < 0.0 {
            let total_shrink_weight: f32 =
                line.iter().map(|&i| items[i].shrink * items[i].basis).sum();
            if total_shrink_weight > 0.0 {
                for (k, &i) in line.iter().enumerate() {
                    let weight = items[i].shrink * items[i].basis;
                    target_outer[k] =
                        (target_outer[k] + free * (weight / total_shrink_weight)).max(0.0);
                }
            }
        }

        let mut placed: Vec<(LayoutNodeId, f32, f32)> = Vec::with_capacity(line.len());
        for (k, &i) in line.iter().enumerate() {
            let item = &items[i];
            let target_border_box = (target_outer[k] - item.margin[1] - item.margin[3]).max(0.0);
            let id = layout_block(
                tree,
                dom,
                styles,
                fonts,
                item.node,
                content_x,
                cursor_y + item.margin[0],
                target_border_box,
            );
            let rect = tree.get(id).rect;
            placed.push((id, rect.width, rect.height));
        }

        let actual_total: f32 = placed.iter().map(|&(_, w, _)| w).sum::<f32>()
            + line
                .iter()
                .map(|&i| items[i].margin[1] + items[i].margin[3])
                .sum::<f32>()
            + main_gap * (line.len().saturating_sub(1)) as f32;
        let remaining = (content_width - actual_total).max(0.0);

        let (leading, between) = justify_offsets(
            container_style.justify_content,
            remaining,
            line.len(),
            main_gap,
        );
        let mut cursor_x = content_x + leading;

        let natural_cross_size = line
            .iter()
            .zip(placed.iter())
            .map(|(&i, &(_, _, h))| items[i].margin[0] + items[i].margin[2] + h)
            .fold(0.0f32, f32::max);
        let line_cross_size = if lines.len() == 1 {
            explicit_content_height.unwrap_or(natural_cross_size)
        } else {
            natural_cross_size
        };

        for (k, &i) in line.iter().enumerate() {
            let item = &items[i];
            let (id, w, h) = placed[k];

            let target_x = cursor_x + item.margin[3];
            let current_rect = tree.get(id).rect;
            let dx = target_x - current_rect.x;

            let cross_avail = line_cross_size - item.margin[0] - item.margin[2];
            let dy = match item.align {
                AlignItems::FlexStart | AlignItems::Stretch => 0.0,
                AlignItems::FlexEnd => (cross_avail - h).max(0.0),
                AlignItems::Center => ((cross_avail - h) / 2.0).max(0.0),
            };

            if dx != 0.0 || dy != 0.0 {
                tree.translate_subtree(id, dx, dy);
            }
            if item.align == AlignItems::Stretch && !item.has_explicit_height {
                tree.set_height(id, cross_avail);
            }

            result_ids.push(id);
            cursor_x = target_x + w + item.margin[1] + between;
        }

        cursor_y += line_cross_size;
        if line_idx + 1 < lines.len() {
            cursor_y += cross_gap;
        }
    }

    (result_ids, cursor_y - content_start_y)
}

fn layout_flex_column(
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

    let main_gap = container_style.row_gap;
    let n = children.len();
    let mut result_ids = Vec::with_capacity(n);
    let mut cursor_y = content_start_y;

    for (idx, &child) in children.iter().enumerate() {
        let cs = &styles[child.index()];
        let margin_top = cs.margin[0].unwrap_or(0.0);
        let margin_right = cs.margin[1].unwrap_or(0.0);
        let margin_bottom = cs.margin[2].unwrap_or(0.0);
        let margin_left = cs.margin[3].unwrap_or(0.0);
        let align = cs.align_self.unwrap_or(container_style.align_items);

        // Stretch (the default) and an explicit width both already resolve
        // correctly by handing the item the full cross size, exactly like
        // an ordinary block child; the other alignments need the item
        // shrunk to its natural width first so there's room to offset it.
        let containing_width_for_item = if align != AlignItems::Stretch && cs.width.is_none() {
            measure_intrinsic_width(dom, styles, fonts, child)
        } else {
            content_width
        };

        let id = layout_block(
            tree,
            dom,
            styles,
            fonts,
            child,
            content_x,
            cursor_y + margin_top,
            containing_width_for_item,
        );
        let rect = tree.get(id).rect;

        let cross_avail = content_width - margin_left - margin_right;
        let dx = match align {
            AlignItems::FlexStart | AlignItems::Stretch => 0.0,
            AlignItems::FlexEnd => (cross_avail - rect.width).max(0.0),
            AlignItems::Center => ((cross_avail - rect.width) / 2.0).max(0.0),
        };
        if dx != 0.0 {
            tree.translate_subtree(id, dx, 0.0);
        }

        cursor_y += margin_top + rect.height + margin_bottom;
        if idx + 1 < n {
            cursor_y += main_gap;
        }
        result_ids.push(id);
    }

    (result_ids, cursor_y - content_start_y)
}

pub(crate) fn layout_flex(
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
    match container_style.flex_direction {
        FlexDirection::Row => layout_flex_row(
            tree,
            dom,
            styles,
            fonts,
            node,
            container_style,
            content_x,
            content_start_y,
            content_width,
        ),
        FlexDirection::Column => layout_flex_column(
            tree,
            dom,
            styles,
            fonts,
            node,
            container_style,
            content_x,
            content_start_y,
            content_width,
        ),
    }
}
