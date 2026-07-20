use placard_font::FontSet;
use placard_html::{Dom, NodeId};
use placard_style::{
    BoxSizing, ComputedStyle, Dimension, Display, FlexDirection, Position, TextAlign,
};
use std::rc::Rc;

use crate::inline::{collect_inline_items, layout_inline_content};
use crate::tree::{BoxKind, BoxNode, LayoutNodeId, LayoutTree, Rect};

pub(crate) fn is_out_of_flow(style: &ComputedStyle) -> bool {
    matches!(style.position, Position::Absolute | Position::Fixed)
}

/// Wide enough that no reasonable badge/card document could ever wrap
/// against it (the render pipeline caps width at 2000px, see the README's
/// 4K pixel budget), so laying a node out against this width and reading
/// back the resulting box's width gives its natural, unwrapped
/// ("shrink-to-fit" / max-content) size.
const UNBOUNDED_WIDTH: f32 = 1_000_000.0;

/// Measures the width a node would take up with no constraint on available
/// space -- used for `flex-basis: auto` sizing of a row-direction flex item
/// that has no explicit `width` of its own.
///
/// This can't just delegate to `layout_block` at a huge containing width:
/// an ordinary block box with no explicit width *fills* whatever containing
/// width it's given (that's the definition of block layout), so it would
/// always measure as `UNBOUNDED_WIDTH` itself rather than shrinking to its
/// content. Instead this recurses the same way `layout_block` dispatches --
/// block children measured and maxed, inline content measured as a single
/// unwrapped line -- without ever writing a used width back into a box.
///
/// Percentage widths on *descendants* of the measured node resolve against
/// `UNBOUNDED_WIDTH` rather than any real containing size, which is wrong
/// in the same way it's wrong for every other CSS engine's naive
/// shrink-to-fit pass; the node being measured itself is only ever called
/// here when it has no percentage/explicit width of its own.
pub(crate) fn measure_intrinsic_width(
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    node: NodeId,
) -> f32 {
    let style = &styles[node.index()];
    let border_h = style.border_width[1] + style.border_width[3];
    let padding_h = style.padding[1] + style.padding[3];

    let content_width = if let Some(dim) = style.width {
        let specified = dim.resolve(UNBOUNDED_WIDTH);
        match style.box_sizing {
            BoxSizing::BorderBox => (specified - padding_h - border_h).max(0.0),
            BoxSizing::ContentBox => specified,
        }
    } else if style.display == Display::Flex {
        measure_flex_intrinsic_width(dom, styles, fonts, node, style)
    } else if is_block_formatting_context(dom, styles, node) {
        dom.children(node)
            .filter(|&child| {
                dom.tag(child).is_some()
                    && styles[child.index()].display != Display::None
                    && !is_out_of_flow(&styles[child.index()])
            })
            .map(|child| {
                let child_style = &styles[child.index()];
                let margin_h =
                    child_style.margin[1].unwrap_or(0.0) + child_style.margin[3].unwrap_or(0.0);
                margin_h + measure_intrinsic_width(dom, styles, fonts, child)
            })
            .fold(0.0f32, f32::max)
    } else {
        let mut items = Vec::new();
        let owner_spans = collect_inline_items(dom, styles, node, &mut items);
        if items.is_empty() {
            0.0
        } else {
            let mut scratch = LayoutTree::empty();
            let (fragment_ids, _height) = layout_inline_content(
                &mut scratch,
                &items,
                &owner_spans,
                fonts,
                0.0,
                0.0,
                UNBOUNDED_WIDTH,
                TextAlign::Left,
            );
            fragment_ids
                .iter()
                .map(|&id| {
                    let b = scratch.get(id);
                    b.rect.x + b.rect.width
                })
                .fold(0.0f32, f32::max)
        }
    };

    content_width + padding_h + border_h
}

/// A flex row's natural width is the sum of its items' natural widths (plus
/// gaps) since they sit side by side; a flex column's is the widest single
/// item, since they stack -- the same row/column split `layout_flex` uses
/// for actual layout, just without ever writing a used size back into a box.
fn measure_flex_intrinsic_width(
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    node: NodeId,
    style: &ComputedStyle,
) -> f32 {
    let items: Vec<NodeId> = dom
        .children(node)
        .filter(|&child| {
            dom.tag(child).is_some()
                && styles[child.index()].display != Display::None
                && !is_out_of_flow(&styles[child.index()])
        })
        .collect();

    if items.is_empty() {
        return 0.0;
    }

    let item_width = |child: &NodeId| {
        let cs = &styles[child.index()];
        let margin_h = cs.margin[1].unwrap_or(0.0) + cs.margin[3].unwrap_or(0.0);
        margin_h + measure_intrinsic_width(dom, styles, fonts, *child)
    };

    if style.flex_direction == FlexDirection::Row {
        let gap_total = style.column_gap * (items.len() - 1) as f32;
        items.iter().map(item_width).sum::<f32>() + gap_total
    } else {
        items.iter().map(item_width).fold(0.0f32, f32::max)
    }
}

fn is_block_formatting_context(dom: &Dom, styles: &[ComputedStyle], node: NodeId) -> bool {
    dom.children(node).any(|child| {
        dom.tag(child).is_some()
            && matches!(
                styles[child.index()].display,
                Display::Block | Display::Flex | Display::Grid
            )
            && !is_out_of_flow(&styles[child.index()])
    })
}

/// Collects `position: absolute` descendants of `node` whose containing
/// block is `node` itself -- i.e. it stops descending as soon as it hits
/// another positioned descendant (relative/absolute/fixed), since that
/// descendant either establishes its own containing block or escapes to
/// the viewport entirely (see `collect_fixed_descendants`).
fn collect_absolute_descendants(
    dom: &Dom,
    styles: &[ComputedStyle],
    node: NodeId,
    out: &mut Vec<NodeId>,
) {
    for child in dom.children(node) {
        if dom.tag(child).is_none() {
            continue;
        }
        let style = &styles[child.index()];
        if style.display == Display::None {
            continue;
        }
        if style.position == Position::Absolute {
            out.push(child);
            continue;
        }
        if style.position != Position::Static {
            continue;
        }
        collect_absolute_descendants(dom, styles, child, out);
    }
}

/// `position: fixed` always resolves against the viewport, regardless of
/// any positioned ancestors in between, so this walks the whole document
/// unconditionally rather than stopping at containing blocks.
fn collect_fixed_descendants(
    dom: &Dom,
    styles: &[ComputedStyle],
    node: NodeId,
    out: &mut Vec<NodeId>,
) {
    for child in dom.children(node) {
        if dom.tag(child).is_none() {
            continue;
        }
        let style = &styles[child.index()];
        if style.display == Display::None {
            continue;
        }
        if style.position == Position::Fixed {
            out.push(child);
        }
        collect_fixed_descendants(dom, styles, child, out);
    }
}

/// Lays out an out-of-flow (`absolute`/`fixed`) child against an already
/// resolved containing block (`cb_*`), then reuses `layout_block`'s
/// existing box-model math (margins, width resolution) unchanged.
///
/// When only one of a pair of inset properties is set (e.g. `right` but
/// not `left`), the box's own size along that axis isn't known until
/// after layout, so it's first laid out at the near edge as a placeholder
/// and then the whole subtree is translated into its final position once
/// its size is known. When both are set, the far edge is instead folded
/// into the containing width passed to `layout_block`, which already
/// stretches auto-width content to fill it.
fn layout_out_of_flow_child(
    tree: &mut LayoutTree,
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    node: NodeId,
    cb_x: f32,
    cb_y: f32,
    cb_width: f32,
    cb_height: f32,
) -> LayoutNodeId {
    let style = &styles[node.index()];
    let left = style.inset[3];
    let right = style.inset[1];
    let top = style.inset[0];
    let bottom = style.inset[2];

    let (init_x, containing_width) = match (left, right) {
        (Some(l), Some(r)) => {
            let l = l.resolve(cb_width);
            let r = r.resolve(cb_width);
            (cb_x + l, (cb_width - l - r).max(0.0))
        }
        (Some(l), None) => (cb_x + l.resolve(cb_width), cb_width),
        (None, _) => (cb_x, cb_width),
    };
    let init_y = match top {
        Some(t) => cb_y + t.resolve(cb_height),
        None => cb_y,
    };

    let child_id = layout_block(
        tree,
        dom,
        styles,
        fonts,
        node,
        init_x,
        init_y,
        containing_width,
    );

    let mut dx = 0.0;
    let mut dy = 0.0;
    if left.is_none() {
        if let Some(r) = right {
            let rect = tree.get(child_id).rect;
            let desired_x = cb_x + cb_width - r.resolve(cb_width) - rect.width;
            dx = desired_x - rect.x;
        }
    }
    if top.is_none() {
        if let Some(b) = bottom {
            let rect = tree.get(child_id).rect;
            let desired_y = cb_y + cb_height - b.resolve(cb_height) - rect.height;
            dy = desired_y - rect.y;
        }
    }
    if dx != 0.0 || dy != 0.0 {
        tree.translate_subtree(child_id, dx, dy);
    }
    child_id
}

/// Lays out and appends the `position: absolute` descendants (and, for the
/// document root, `position: fixed` descendants too) whose containing
/// block is this box, once its own border box is finalized. Sorted by
/// `z-index` (ties broken by DOM order) so later painting -- a flat
/// depth-first walk of `children` -- draws higher stacked items on top.
fn layout_positioned_children(
    tree: &mut LayoutTree,
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    node: NodeId,
    is_root: bool,
    cb_x: f32,
    cb_y: f32,
    cb_width: f32,
    cb_height: f32,
    children: &mut Vec<LayoutNodeId>,
) {
    let mut positioned = Vec::new();
    collect_absolute_descendants(dom, styles, node, &mut positioned);
    if is_root {
        collect_fixed_descendants(dom, styles, node, &mut positioned);
    }
    positioned.sort_by_key(|&n| styles[n.index()].z_index.unwrap_or(0));

    for positioned_node in positioned {
        let id = layout_out_of_flow_child(
            tree,
            dom,
            styles,
            fonts,
            positioned_node,
            cb_x,
            cb_y,
            cb_width,
            cb_height,
        );
        children.push(id);
    }
}

pub(crate) fn layout_block(
    tree: &mut LayoutTree,
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    node: NodeId,
    containing_x: f32,
    containing_y: f32,
    containing_width: f32,
) -> LayoutNodeId {
    let style = Rc::new(styles[node.index()].clone());

    let margin_top = style.margin[0].unwrap_or(0.0);
    let margin_right_auto = style.margin[1].is_none();
    let margin_left_auto = style.margin[3].is_none();
    let mut margin_left = style.margin[3].unwrap_or(0.0);

    let border_h = style.border_width[1] + style.border_width[3];
    let padding_h = style.padding[1] + style.padding[3];

    let content_width = match style.width {
        Some(dim) => {
            let specified = dim.resolve(containing_width);
            match style.box_sizing {
                BoxSizing::BorderBox => (specified - padding_h - border_h).max(0.0),
                BoxSizing::ContentBox => specified,
            }
        }
        None => {
            let ml = if margin_left_auto { 0.0 } else { margin_left };
            let mr = if margin_right_auto {
                0.0
            } else {
                style.margin[1].unwrap_or(0.0)
            };
            (containing_width - ml - mr - border_h - padding_h).max(0.0)
        }
    };

    if style.width.is_some() && (margin_left_auto || margin_right_auto) {
        let border_box_width = content_width + padding_h + border_h;
        let remaining = (containing_width - border_box_width).max(0.0);
        if margin_left_auto && margin_right_auto {
            margin_left = remaining / 2.0;
        } else if margin_left_auto {
            margin_left = remaining;
        }
    }

    let border_box_x = containing_x + margin_left;
    let border_box_y = containing_y + margin_top;
    let content_x = border_box_x + style.border_width[3] + style.padding[3];
    let content_start_y = border_box_y + style.border_width[0] + style.padding[0];

    let mut children;
    let mut content_height;

    if style.display == Display::Flex {
        let (flex_children, height) = crate::flex::layout_flex(
            tree,
            dom,
            styles,
            fonts,
            node,
            &style,
            content_x,
            content_start_y,
            content_width,
        );
        content_height = height;
        children = flex_children;
    } else if style.display == Display::Grid {
        let (grid_children, height) = crate::grid::layout_grid(
            tree,
            dom,
            styles,
            fonts,
            node,
            &style,
            content_x,
            content_start_y,
            content_width,
        );
        content_height = height;
        children = grid_children;
    } else if is_block_formatting_context(dom, styles, node) {
        let mut boxes = Vec::new();
        let mut cursor_y = content_start_y;
        for child in dom.children(node) {
            if dom.tag(child).is_none() {
                continue;
            }
            if styles[child.index()].display == Display::None {
                continue;
            }
            if is_out_of_flow(&styles[child.index()]) {
                continue;
            }
            let child_id = layout_block(
                tree,
                dom,
                styles,
                fonts,
                child,
                content_x,
                cursor_y,
                content_width,
            );
            let child_box = tree.get(child_id);
            let child_margin_top = child_box.style.margin[0].unwrap_or(0.0);
            let child_margin_bottom = child_box.style.margin[2].unwrap_or(0.0);
            cursor_y += child_margin_top + child_box.rect.height + child_margin_bottom;
            boxes.push(child_id);
        }
        content_height = cursor_y - content_start_y;
        children = boxes;
    } else {
        let mut items = Vec::new();
        let owner_spans = collect_inline_items(dom, styles, node, &mut items);
        let (fragment_ids, height) = layout_inline_content(
            tree,
            &items,
            &owner_spans,
            fonts,
            content_x,
            content_start_y,
            content_width,
            style.text_align,
        );
        content_height = height;
        children = fragment_ids;
    }

    if let Some(Dimension::Px(v)) = style.height {
        content_height = match style.box_sizing {
            BoxSizing::BorderBox => (v
                - style.padding[0]
                - style.padding[2]
                - style.border_width[0]
                - style.border_width[2])
                .max(0.0),
            BoxSizing::ContentBox => v,
        };
    }

    let border_box_width = content_width
        + style.padding[1]
        + style.padding[3]
        + style.border_width[1]
        + style.border_width[3];
    let border_box_height = content_height
        + style.padding[0]
        + style.padding[2]
        + style.border_width[0]
        + style.border_width[2];

    // `position: relative` shifts the box (and everything already laid out
    // inside it) without disturbing normal flow: siblings were already
    // placed using this box's un-shifted height, so translating after the
    // fact is exactly equivalent to -- and simpler than -- threading the
    // offset through the whole subtree's layout.
    let (dx, dy) = if style.position == Position::Relative {
        let dx = match (style.inset[3], style.inset[1]) {
            (Some(l), _) => l.resolve(containing_width),
            (None, Some(r)) => -r.resolve(containing_width),
            (None, None) => 0.0,
        };
        let dy = match (style.inset[0], style.inset[2]) {
            (Some(t), _) => t.resolve(0.0),
            (None, Some(b)) => -b.resolve(0.0),
            (None, None) => 0.0,
        };
        (dx, dy)
    } else {
        (0.0, 0.0)
    };
    if dx != 0.0 || dy != 0.0 {
        for &child_id in &children {
            tree.translate_subtree(child_id, dx, dy);
        }
    }

    let final_x = border_box_x + dx;
    let final_y = border_box_y + dy;

    if style.position != Position::Static {
        let cb_x = final_x + style.border_width[3];
        let cb_y = final_y + style.border_width[0];
        let cb_width = border_box_width - style.border_width[1] - style.border_width[3];
        let cb_height = border_box_height - style.border_width[0] - style.border_width[2];
        layout_positioned_children(
            tree,
            dom,
            styles,
            fonts,
            node,
            false,
            cb_x,
            cb_y,
            cb_width,
            cb_height,
            &mut children,
        );
    }

    tree.push(BoxNode {
        kind: BoxKind::Block,
        rect: Rect {
            x: final_x,
            y: final_y,
            width: border_box_width,
            height: border_box_height,
        },
        style,
        children,
    })
}

fn has_explicit_html_or_body(dom: &Dom, document: NodeId) -> bool {
    dom.children(document)
        .any(|c| matches!(dom.tag(c), Some("html") | Some("body")))
}

const UA_BODY_MARGIN: f32 = 8.0;

/// The natural ("shrink-to-fit") width of the whole document -- the width
/// `build` would need in order for its top-level content to lay out without
/// wrapping, honoring the same body-margin rules `build` itself uses when
/// deciding the document's origin and available width.
pub fn measure_document_width(dom: &Dom, styles: &[ComputedStyle], fonts: &FontSet) -> f32 {
    let document = dom.root();

    let intrinsic = dom
        .children(document)
        .filter(|&child| {
            dom.tag(child).is_some()
                && styles[child.index()].display != Display::None
                && !is_out_of_flow(&styles[child.index()])
        })
        .map(|child| {
            let child_style = &styles[child.index()];
            let margin_h =
                child_style.margin[1].unwrap_or(0.0) + child_style.margin[3].unwrap_or(0.0);
            margin_h + measure_intrinsic_width(dom, styles, fonts, child)
        })
        .fold(0.0f32, f32::max);

    if has_explicit_html_or_body(dom, document) {
        intrinsic
    } else {
        intrinsic + 2.0 * UA_BODY_MARGIN
    }
}

pub fn build(
    dom: &Dom,
    styles: &[ComputedStyle],
    fonts: &FontSet,
    viewport_width: f32,
) -> LayoutTree {
    let mut tree = LayoutTree::empty();
    let document = dom.root();

    let (origin_x, origin_y, content_width, bottom_inset) =
        if has_explicit_html_or_body(dom, document) {
            (0.0, 0.0, viewport_width, 0.0)
        } else {
            let default_body_margin = UA_BODY_MARGIN;
            (
                default_body_margin,
                default_body_margin,
                (viewport_width - 2.0 * default_body_margin).max(0.0),
                default_body_margin,
            )
        };

    let mut children = Vec::new();
    let mut cursor_y = origin_y;
    for child in dom.children(document) {
        if dom.tag(child).is_none() {
            continue;
        }
        if styles[child.index()].display == Display::None {
            continue;
        }
        if is_out_of_flow(&styles[child.index()]) {
            continue;
        }
        let child_id = layout_block(
            &mut tree,
            dom,
            styles,
            fonts,
            child,
            origin_x,
            cursor_y,
            content_width,
        );
        let child_box = tree.get(child_id);
        let child_margin_top = child_box.style.margin[0].unwrap_or(0.0);
        let child_margin_bottom = child_box.style.margin[2].unwrap_or(0.0);
        cursor_y += child_margin_top + child_box.rect.height + child_margin_bottom;
        children.push(child_id);
    }
    cursor_y += bottom_inset;

    // The document root is the initial containing block: origin (0, 0),
    // full viewport width, and content-derived height -- the same
    // reference frame both `position: fixed` and any `position: absolute`
    // descendant with no positioned ancestor resolve against.
    layout_positioned_children(
        &mut tree,
        dom,
        styles,
        fonts,
        document,
        true,
        0.0,
        0.0,
        viewport_width,
        cursor_y,
        &mut children,
    );

    let root_id = tree.push(BoxNode {
        kind: BoxKind::Block,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: viewport_width,
            height: cursor_y,
        },
        style: Rc::new(ComputedStyle::initial()),
        children,
    });
    tree.set_root(root_id);
    tree
}
