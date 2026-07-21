use placard_font::{
    Font, FontFamily as FontDbFamily, FontSet, FontStyle as FontDbStyle, FontWeight as FontDbWeight,
};
use placard_html::{Dom, NodeData, NodeId};
use placard_style::{
    BorderStyle, ComputedStyle, Display, FontFamily, FontStyle, FontWeight, LineHeight, Position,
    TextAlign,
};
use std::rc::Rc;

use crate::tree::{BoxKind, BoxNode, LayoutNodeId, LayoutTree, Rect};

pub(crate) fn resolve_font<'a>(fonts: &'a FontSet, style: &ComputedStyle) -> &'a Font {
    let families: Vec<FontDbFamily> = style
        .font_family
        .iter()
        .map(|f| match f {
            FontFamily::SansSerif => FontDbFamily::SansSerif,
            FontFamily::Serif => FontDbFamily::Serif,
            FontFamily::Monospace => FontDbFamily::Monospace,
            FontFamily::Named(name) => FontDbFamily::Named(name.clone()),
        })
        .collect();
    let weight = match style.font_weight {
        FontWeight::Normal => FontDbWeight::Normal,
        FontWeight::Bold => FontDbWeight::Bold,
    };
    let style = match style.font_style {
        FontStyle::Normal => FontDbStyle::Normal,
        FontStyle::Italic => FontDbStyle::Italic,
    };
    fonts.resolve(&families, weight, style)
}

pub(crate) struct InlineItem {
    word: String,
    style: Rc<ComputedStyle>,
    space_before: bool,
}

/// A contiguous span of `InlineItem`s (in document order) descending from a
/// single element that has a visible border or background. Nested owners
/// span a subrange of their ancestor's -- painting them in the order
/// collected (ancestors before descendants) layers correctly, since an
/// ancestor's box always covers a superset of its descendants' area.
pub(crate) struct OwnerSpan {
    style: Rc<ComputedStyle>,
    start: usize,
    end: usize,
}

pub(crate) fn collect_inline_items(
    dom: &Dom,
    styles: &[ComputedStyle],
    node: NodeId,
    out: &mut Vec<InlineItem>,
) -> Vec<OwnerSpan> {
    let mut pending_space = false;
    let mut owners = Vec::new();
    collect_inline_items_rec(dom, styles, node, out, &mut pending_space, &mut owners);
    owners
}

fn collect_inline_items_rec(
    dom: &Dom,
    styles: &[ComputedStyle],
    node: NodeId,
    out: &mut Vec<InlineItem>,
    pending_space: &mut bool,
    owners: &mut Vec<OwnerSpan>,
) {
    for child in dom.children(node) {
        match dom.data(child) {
            NodeData::Text(text) => {
                let style = Rc::new(styles[node.index()].clone());
                if text.starts_with(char::is_whitespace) {
                    *pending_space = true;
                }
                let mut has_word = false;
                for word in text.split_whitespace() {
                    out.push(InlineItem {
                        word: word.to_string(),
                        style: style.clone(),
                        space_before: *pending_space,
                    });
                    *pending_space = true;
                    has_word = true;
                }
                if has_word {
                    *pending_space = text.ends_with(char::is_whitespace);
                }
            }
            NodeData::Element { .. } => {
                let child_style = &styles[child.index()];
                if child_style.display == Display::None {
                    continue;
                }
                if matches!(child_style.position, Position::Absolute | Position::Fixed) {
                    continue;
                }
                let has_visible_box = child_style.background_color.a > 0
                    || child_style.border_width.iter().any(|&w| w > 0.0);

                let start = out.len();
                let owner_idx = if has_visible_box {
                    owners.push(OwnerSpan {
                        style: Rc::new(child_style.clone()),
                        start,
                        end: start,
                    });
                    Some(owners.len() - 1)
                } else {
                    None
                };

                collect_inline_items_rec(dom, styles, child, out, pending_space, owners);

                if out.len() == start && has_visible_box {
                    out.push(InlineItem {
                        word: String::new(),
                        style: Rc::new(child_style.clone()),
                        space_before: *pending_space,
                    });
                    *pending_space = false;
                }

                if let Some(idx) = owner_idx {
                    owners[idx].end = out.len();
                }
            }
            NodeData::Document => {}
        }
    }
}

fn measure_text(font: &Font, text: &str, scale: f32, letter_spacing: f32) -> f32 {
    let mut width = 0.0f32;
    let mut count = 0usize;
    for c in text.chars() {
        if let Some(g) = font.glyph_id_for_char(c) {
            width += font.advance_width(g) as f32 * scale;
            count += 1;
        }
    }
    width + letter_spacing * count as f32
}

fn resolved_line_height(style: &ComputedStyle) -> f32 {
    match style.line_height {
        LineHeight::Normal => style.font_size * 1.2,
        LineHeight::Number(n) => style.font_size * n,
        LineHeight::Px(px) => px,
    }
}

fn content_box_height(font: &Font, scale: f32) -> f32 {
    (font.ascender() as f32 - font.descender() as f32) * scale
}

fn effective_border(style: &ComputedStyle, idx: usize) -> f32 {
    if style.border_style[idx] == BorderStyle::Solid {
        style.border_width[idx]
    } else {
        0.0
    }
}

fn horizontal_edge_space(style: &ComputedStyle, side: usize) -> f32 {
    style.margin[side].unwrap_or(0.0) + effective_border(style, side) + style.padding[side]
}

fn horizontal_box_extent(style: &ComputedStyle, side: usize) -> f32 {
    effective_border(style, side) + style.padding[side]
}

struct PlacedWord {
    word: String,
    style: Rc<ComputedStyle>,
    rect: Rect,
}

pub(crate) fn layout_inline_content(
    tree: &mut LayoutTree,
    items: &[InlineItem],
    owner_spans: &[OwnerSpan],
    fonts: &FontSet,
    origin_x: f32,
    origin_y: f32,
    available_width: f32,
    text_align: TextAlign,
) -> (Vec<LayoutNodeId>, f32) {
    if items.is_empty() {
        return (Vec::new(), 0.0);
    }

    // Extra space reserved at an item's edge for every owner box that starts
    // or ends there -- an item can be the first/last descendant of several
    // nested owners at once, so these accumulate rather than replace.
    let mut left_extra = vec![0.0f32; items.len()];
    let mut right_extra = vec![0.0f32; items.len()];
    for owner in owner_spans {
        if owner.start < owner.end {
            left_extra[owner.start] += horizontal_edge_space(&owner.style, 3);
            right_extra[owner.end - 1] += horizontal_edge_space(&owner.style, 1);
        }
    }

    let mut placed: Vec<PlacedWord> = Vec::with_capacity(items.len());
    let mut cursor_x = origin_x;
    let mut cursor_y = origin_y;
    let mut line_height = 0.0f32;
    let mut any_on_line = false;
    let mut line_bounds: Vec<(usize, usize, f32)> = Vec::new();
    let mut line_heights: Vec<f32> = Vec::new();
    let mut current_line_start = 0usize;

    for (i, item) in items.iter().enumerate() {
        let font = resolve_font(fonts, &item.style);
        let scale = item.style.font_size / font.units_per_em() as f32;
        let word_width = measure_text(font, &item.word, scale, item.style.letter_spacing);
        let space_width = measure_text(font, " ", scale, item.style.letter_spacing);
        let content_height = content_box_height(font, scale);
        let line_box_height = resolved_line_height(&item.style).max(content_height);

        let left_edge = left_extra[i];
        let right_edge = right_extra[i];

        let space_before = any_on_line && item.space_before;
        let needed =
            (if space_before { space_width } else { 0.0 }) + left_edge + word_width + right_edge;

        if any_on_line && cursor_x + needed > origin_x + available_width {
            line_bounds.push((current_line_start, i, cursor_x));
            line_heights.push(line_height);
            cursor_y += line_height;
            cursor_x = origin_x;
            line_height = 0.0;
            any_on_line = false;
            current_line_start = i;
        }

        let space_before = any_on_line && item.space_before;
        let word_x = (if space_before {
            cursor_x + space_width
        } else {
            cursor_x
        }) + left_edge;

        placed.push(PlacedWord {
            word: item.word.clone(),
            style: item.style.clone(),
            rect: Rect {
                x: word_x,
                y: cursor_y,
                width: word_width,
                height: content_height,
            },
        });

        cursor_x = word_x + word_width + right_edge;
        line_height = line_height.max(line_box_height);
        any_on_line = true;
    }
    line_bounds.push((current_line_start, placed.len(), cursor_x));
    line_heights.push(line_height);

    for (line_idx, (start, end, _)) in line_bounds.iter().enumerate() {
        let final_height = line_heights[line_idx];
        for w in &mut placed[*start..*end] {
            let half_leading = ((final_height - w.rect.height) / 2.0).max(0.0);
            w.rect.y += half_leading;
        }
    }

    if text_align != TextAlign::Left {
        for (start, end, line_end_x) in &line_bounds {
            let line_width = line_end_x - origin_x;
            let offset = match text_align {
                TextAlign::Left => 0.0,
                TextAlign::Center => ((available_width - line_width) / 2.0).max(0.0),
                TextAlign::Right => (available_width - line_width).max(0.0),
            };
            if offset > 0.0 {
                for w in &mut placed[*start..*end] {
                    w.rect.x += offset;
                }
            }
        }
    }

    let total_height = (cursor_y - origin_y) + line_height;

    let mut result_ids = Vec::new();

    // Each owner contributes one box per line its content appears on, so an
    // owner whose descendants wrap across several lines still fragments
    // correctly -- only the first fragment gets the left edge and only the
    // last gets the right edge; top/bottom apply to every fragment.
    for owner in owner_spans {
        if owner.start >= owner.end {
            continue;
        }
        let mut i = owner.start;
        while i < owner.end {
            let mut j = i + 1;
            while j < owner.end && placed[j].rect.y == placed[i].rect.y {
                j += 1;
            }

            let left_extent = if i == owner.start {
                horizontal_box_extent(&owner.style, 3)
            } else {
                0.0
            };
            let right_extent = if j == owner.end {
                horizontal_box_extent(&owner.style, 1)
            } else {
                0.0
            };
            let top_extent = effective_border(&owner.style, 0) + owner.style.padding[0];
            let bottom_extent = effective_border(&owner.style, 2) + owner.style.padding[2];

            let run = &placed[i..j];
            let min_x = run[0].rect.x - left_extent;
            let max_x = run.last().unwrap().rect.x + run.last().unwrap().rect.width + right_extent;
            let content_height = run.iter().map(|w| w.rect.height).fold(0.0f32, f32::max);

            let bg_id = tree.push(BoxNode {
                kind: BoxKind::InlineBackground,
                rect: Rect {
                    x: min_x,
                    y: run[0].rect.y - top_extent,
                    width: max_x - min_x,
                    height: content_height + top_extent + bottom_extent,
                },
                style: owner.style.clone(),
                children: Vec::new(),
            });
            result_ids.push(bg_id);

            i = j;
        }
    }

    for w in &placed {
        let id = tree.push(BoxNode {
            kind: BoxKind::Text {
                content: w.word.clone(),
            },
            rect: w.rect,
            style: w.style.clone(),
            children: Vec::new(),
        });
        result_ids.push(id);
    }

    (result_ids, total_height)
}
