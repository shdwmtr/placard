mod block;
mod flex;
mod grid;
mod inline;
mod tree;

pub use block::{build, measure_document_width};
pub use placard_style::{
    AlignItems, BorderStyle, BoxSizing, Color, ComputedStyle, Dimension, Display, FlexDirection,
    FlexWrap, FontFamily, FontStyle, FontWeight, JustifyContent, LineHeight, Position, Side,
    TextAlign, TrackSize,
};
pub use tree::{BoxKind, BoxNode, LayoutNodeId, LayoutTree, Rect};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::measure_intrinsic_width;
    use placard_font::{Font, FontSet};

    fn test_font() -> FontSet {
        let data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
            .expect("failed to read font");
        FontSet::new(Font::parse(&data).expect("failed to parse font"))
    }

    #[test]
    fn margin_padding_border_produce_correct_border_box() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"box\"></div>");
        let sheet = placard_css::parse(
            "div.box {
                margin-top: 5px; margin-right: 6px; margin-bottom: 7px; margin-left: 8px;
                padding-top: 1px; padding-right: 2px; padding-bottom: 3px; padding-left: 4px;
                border-top-width: 1px; border-right-width: 1px; border-bottom-width: 1px; border-left-width: 1px;
            }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 100.0);

        let root = tree.get(tree.root());
        let div_id = root.children[0];
        let div = tree.get(div_id);

        assert_eq!(div.rect.x, 16.0);
        assert_eq!(div.rect.y, 13.0);

        assert_eq!(div.rect.width, 70.0);

        assert_eq!(div.rect.height, 6.0);
    }

    #[test]
    fn block_children_stack_vertically_with_correct_offsets() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"a\"></div><div class=\"b\"></div>");
        let sheet = placard_css::parse(
            "div.a { margin-top: 10px; margin-bottom: 4px; height: 20px; }
             div.b { margin-top: 6px; height: 30px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 100.0);

        let root = tree.get(tree.root());
        let a = tree.get(root.children[0]);
        let b = tree.get(root.children[1]);

        assert_eq!(a.rect.y, 18.0);
        assert_eq!(a.rect.height, 20.0);

        assert_eq!(b.rect.y, 48.0);
        assert_eq!(b.rect.height, 30.0);
    }

    #[test]
    fn display_none_element_produces_no_box_and_no_gap() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"a\"></div><div class=\"hidden\"></div><div class=\"c\"></div>",
        );
        let sheet = placard_css::parse(
            "div.a { height: 10px; }
             div.hidden { display: none; height: 500px; }
             div.c { height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 100.0);

        let root = tree.get(tree.root());
        assert_eq!(root.children.len(), 2);
        let c = tree.get(root.children[1]);

        assert_eq!(c.rect.y, 18.0);
    }

    #[test]
    fn explicit_width_overrides_auto_width() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"box\"></div>");
        let sheet = placard_css::parse("div.box { width: 42px; }");
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 500.0);

        let root = tree.get(tree.root());
        let div = tree.get(root.children[0]);
        assert_eq!(div.rect.width, 42.0);
    }

    #[test]
    fn whitespace_only_inline_element_still_produces_a_background_box() {
        let font = test_font();
        let dom =
            placard_html::parse("<div class=\"wrap\"><span class=\"dot\">&nbsp;</span></div>");
        let sheet = placard_css::parse(
            "div.wrap { width: 100px; }
             span.dot { background-color: red; padding: 5px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 200.0);

        let root = tree.get(tree.root());
        let wrap = tree.get(root.children[0]);
        let bg = wrap
            .children
            .iter()
            .map(|&id| tree.get(id))
            .find(|b| matches!(b.kind, BoxKind::InlineBackground))
            .expect("whitespace-only span should still yield an InlineBackground box");

        assert_eq!(bg.rect.width, 10.0);
        assert_eq!(bg.rect.height, 27.875);
    }

    #[test]
    fn whitespace_between_inline_siblings_is_not_double_spaced() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"wrap\"><span>a</span> <span>b</span></div>");
        let sheet = placard_css::parse("div.wrap { width: 100px; }");
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 200.0);

        let root = tree.get(tree.root());
        let wrap = tree.get(root.children[0]);
        let words: Vec<&BoxNode> = wrap
            .children
            .iter()
            .map(|&id| tree.get(id))
            .filter(|b| matches!(&b.kind, BoxKind::Text { content } if !content.is_empty()))
            .collect();

        assert_eq!(words.len(), 2);
        let gap = words[1].rect.x - (words[0].rect.x + words[0].rect.width);
        let space_width = {
            let f = font.get(
                placard_font::FontFamily::SansSerif,
                placard_font::FontWeight::Normal,
                placard_font::FontStyle::Normal,
            );
            let scale = 16.0 / f.units_per_em() as f32;
            f.glyph_id_for_char(' ')
                .map(|g| f.advance_width(g) as f32 * scale)
                .unwrap_or(0.0)
        };
        assert_eq!(gap, space_width);
    }

    #[test]
    fn inherited_line_height_does_not_inflate_inline_background_box() {
        let font = test_font();
        let dom = placard_html::parse("<p class=\"wrap\"><span class=\"chip\">hi</span></p>");
        let sheet = placard_css::parse(
            "p.wrap { width: 100px; line-height: 2.6em; }
             span.chip { font-size: 12px; padding: 8px 16px; border: 2px solid red; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 200.0);

        let root = tree.get(tree.root());
        let wrap = tree.get(root.children[0]);
        let bg = wrap
            .children
            .iter()
            .map(|&id| tree.get(id))
            .find(|b| matches!(b.kind, BoxKind::InlineBackground))
            .expect("chip should yield an InlineBackground box");

        let f = font.get(
            placard_font::FontFamily::SansSerif,
            placard_font::FontWeight::Normal,
            placard_font::FontStyle::Normal,
        );
        let scale = 12.0 / f.units_per_em() as f32;
        let content_height = (f.ascender() as f32 - f.descender() as f32) * scale;
        let expected = content_height + 2.0 * (8.0 + 2.0);
        assert_eq!(bg.rect.height, expected);
        assert!(bg.rect.height < 41.6);
    }

    #[test]
    fn adjacent_inline_elements_with_no_source_whitespace_touch() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"wrap\"><span class=\"a\">x</span><span class=\"b\">y</span></div>",
        );
        let sheet = placard_css::parse("div.wrap { width: 100px; }");
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 200.0);

        let root = tree.get(tree.root());
        let wrap = tree.get(root.children[0]);
        let words: Vec<&BoxNode> = wrap
            .children
            .iter()
            .map(|&id| tree.get(id))
            .filter(|b| matches!(&b.kind, BoxKind::Text { content } if !content.is_empty()))
            .collect();

        assert_eq!(words.len(), 2);
        let gap = words[1].rect.x - (words[0].rect.x + words[0].rect.width);
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn relative_offset_shifts_box_without_disturbing_siblings() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"a\"></div><div class=\"b\"></div>");
        let sheet = placard_css::parse(
            "div.a { height: 20px; position: relative; top: 5px; left: 3px; }
             div.b { height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 100.0);

        let root = tree.get(tree.root());
        let a = tree.get(root.children[0]);
        let b = tree.get(root.children[1]);

        assert_eq!(a.rect.x, 11.0);
        assert_eq!(a.rect.y, 13.0);
        assert_eq!(a.rect.height, 20.0);

        assert_eq!(b.rect.y, 28.0);
        assert_eq!(b.rect.height, 10.0);
    }

    #[test]
    fn absolute_child_resolves_against_positioned_ancestor_padding_box() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"wrap\"><div class=\"abs\"></div></div>");
        let sheet = placard_css::parse(
            "div.wrap { position: relative; width: 100px; height: 50px; border: 2px solid black; }
             div.abs { position: absolute; top: 10px; left: 6px; width: 20px; height: 8px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 300.0);

        let root = tree.get(tree.root());
        let wrap = tree.get(root.children[0]);

        assert_eq!(wrap.children.len(), 1);
        let abs = tree.get(wrap.children[0]);

        // wrap's border box starts at (8, 8) (default body margin); its
        // padding box (containing block for `abs`) is inset by the 2px border.
        assert_eq!(abs.rect.x, 8.0 + 2.0 + 6.0);
        assert_eq!(abs.rect.y, 8.0 + 2.0 + 10.0);
        assert_eq!(abs.rect.width, 20.0);
        assert_eq!(abs.rect.height, 8.0);
    }

    #[test]
    fn absolute_child_falls_back_to_document_root_without_positioned_ancestor() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"wrap\"><div class=\"abs\"></div></div>");
        let sheet = placard_css::parse(
            "div.wrap { width: 100px; height: 50px; }
             div.abs { position: absolute; top: 4px; right: 4px; width: 20px; height: 8px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 300.0);

        let root = tree.get(tree.root());
        // Not a descendant of `wrap` in the tree -- it attaches to the root,
        // the nearest positioned (or initial) containing block.
        assert_eq!(root.children.len(), 2);
        let abs = tree.get(root.children[1]);

        assert_eq!(abs.rect.x, 300.0 - 4.0 - 20.0);
        assert_eq!(abs.rect.y, 4.0);
    }

    #[test]
    fn fixed_child_ignores_intervening_positioned_ancestor() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"wrap\"><div class=\"fixed\"></div></div>");
        let sheet = placard_css::parse(
            "div.wrap { position: relative; top: 40px; left: 40px; width: 100px; height: 50px; }
             div.fixed { position: fixed; top: 2px; left: 2px; width: 10px; height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 300.0);

        let root = tree.get(tree.root());
        assert_eq!(root.children.len(), 2);
        let fixed = tree.get(root.children[1]);

        // Resolves against the viewport, not `wrap`'s shifted padding box.
        assert_eq!(fixed.rect.x, 2.0);
        assert_eq!(fixed.rect.y, 2.0);
    }

    #[test]
    fn measure_intrinsic_width_matches_natural_unwrapped_text_size() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"a\">Hello</div>");
        let styles = placard_style::compute(&dom, &placard_css::parse(""));
        let div = dom.first_child(dom.root()).unwrap();

        let width = measure_intrinsic_width(&dom, &styles, &font, div);

        let f = font.get(
            placard_font::FontFamily::SansSerif,
            placard_font::FontWeight::Normal,
            placard_font::FontStyle::Normal,
        );
        let scale = 16.0 / f.units_per_em() as f32;
        let expected: f32 = "Hello"
            .chars()
            .filter_map(|c| f.glyph_id_for_char(c))
            .map(|g| f.advance_width(g) as f32 * scale)
            .sum();

        assert_eq!(width, expected);
    }

    #[test]
    fn letter_spacing_widens_measured_text_by_a_gap_per_character() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"a\">Hello</div>");
        let sheet = placard_css::parse("div.a { letter-spacing: 4px; }");
        let styles = placard_style::compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();

        let width = measure_intrinsic_width(&dom, &styles, &font, div);

        let f = font.get(
            placard_font::FontFamily::SansSerif,
            placard_font::FontWeight::Normal,
            placard_font::FontStyle::Normal,
        );
        let scale = 16.0 / f.units_per_em() as f32;
        let base_width: f32 = "Hello"
            .chars()
            .filter_map(|c| f.glyph_id_for_char(c))
            .map(|g| f.advance_width(g) as f32 * scale)
            .sum();

        assert_eq!(width, base_width + 4.0 * 5.0);
    }

    #[test]
    fn measure_intrinsic_width_of_block_children_takes_the_widest_including_margin() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"wrap\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.a { width: 20px; margin-left: 40px; }
             div.b { width: 50px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let wrap = dom.first_child(dom.root()).unwrap();

        let width = measure_intrinsic_width(&dom, &styles, &font, wrap);

        // a: 40px margin + 20px width = 60, wider than b's bare 50px.
        assert_eq!(width, 60.0);
    }

    #[test]
    fn measure_intrinsic_width_honors_explicit_width() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"a\"></div>");
        let sheet = placard_css::parse("div.a { width: 42px; }");
        let styles = placard_style::compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();

        let width = measure_intrinsic_width(&dom, &styles, &font, div);

        assert_eq!(width, 42.0);
    }

    #[test]
    fn measure_intrinsic_width_of_flex_row_sums_items_and_gaps() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"row\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.row { display: flex; column-gap: 10px; }
             div.a { width: 50px; }
             div.b { width: 60px; margin-left: 5px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let row = dom.first_child(dom.root()).unwrap();

        let width = measure_intrinsic_width(&dom, &styles, &font, row);

        // 50 + (5 + 60) + one 10px gap between the two items.
        assert_eq!(width, 125.0);
    }

    #[test]
    fn max_extent_x_ignores_a_transparent_container_filled_wider_than_its_content() {
        let font = test_font();
        // A flex row with no `justify-content` packs its (narrower) items
        // to the left; building it against a container wider than their
        // natural sum -- exactly what an auto-width safety margin does --
        // leaves the row itself spanning the full given width even though
        // nothing is painted past where the items end.
        let dom = placard_html::parse(
            "<body style=\"margin: 0\"><div class=\"row\"><div class=\"a\"></div></div></body>",
        );
        let sheet = placard_css::parse(
            "div.row { display: flex; }
             div.a { width: 40px; height: 10px; background-color: green; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 200.0);

        assert_eq!(tree.max_extent_x(), 40.0);
    }

    #[test]
    fn measure_intrinsic_width_of_flex_column_takes_the_widest_item() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"col\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.col { display: flex; flex-direction: column; }
             div.a { width: 50px; }
             div.b { width: 60px; margin-left: 5px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let col = dom.first_child(dom.root()).unwrap();

        let width = measure_intrinsic_width(&dom, &styles, &font, col);

        assert_eq!(width, 65.0);
    }

    #[test]
    fn document_width_of_flex_badge_matches_whether_body_is_explicit_or_not() {
        let font = test_font();
        let css = "div.wrap { display: flex; }
                   div.a { width: 50px; }
                   div.b { width: 60px; }";
        let sheet = placard_css::parse(css);

        let with_body = placard_html::parse(
            "<body><div class=\"wrap\"><div class=\"a\"></div><div class=\"b\"></div></div></body>",
        );
        let styles = placard_style::compute(&with_body, &sheet);
        let width = measure_document_width(&with_body, &styles, &font);
        // 8px UA body margin on each side + 110px of content.
        assert_eq!(width, 126.0);

        let without_body = placard_html::parse(
            "<div class=\"wrap\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        );
        let styles = placard_style::compute(&without_body, &sheet);
        let width = measure_document_width(&without_body, &styles, &font);
        assert_eq!(width, 126.0);
    }

    #[test]
    fn flex_row_justify_content_space_between_spreads_fixed_width_items() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"row\"><div class=\"a\"></div><div class=\"b\"></div><div class=\"c\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.row { display: flex; justify-content: space-between; width: 300px; }
             div.a, div.b, div.c { width: 50px; height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let row = tree.get(root.children[0]);
        let a = tree.get(row.children[0]);
        let b = tree.get(row.children[1]);
        let c = tree.get(row.children[2]);

        assert_eq!(a.rect.x, 8.0);
        assert_eq!(b.rect.x, 133.0);
        assert_eq!(c.rect.x, 258.0);
    }

    #[test]
    fn flex_grow_distributes_remaining_width_to_basis_auto_item() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"row\"><div class=\"x\"></div><div class=\"y\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.row { display: flex; width: 200px; }
             div.x { flex: 1; height: 10px; }
             div.y { width: 50px; height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let row = tree.get(root.children[0]);
        let x = tree.get(row.children[0]);
        let y = tree.get(row.children[1]);

        // x has no explicit width (flex-basis: 0%), so it absorbs all 150px
        // of free space; y keeps its own explicit 50px.
        assert_eq!(x.rect.width, 150.0);
        assert_eq!(y.rect.width, 50.0);
        assert_eq!(y.rect.x, 8.0 + 150.0);
    }

    #[test]
    fn flex_shrink_compresses_basis_proportionally_on_overflow() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"row\"><div class=\"p\"></div><div class=\"q\"></div></div>",
        );
        let sheet = placard_css::parse(
            // flex-basis (not width) so the containing-width the shrunk
            // size is passed as actually takes effect on the item.
            "div.row { display: flex; width: 100px; }
             div.p, div.q { flex-basis: 80px; flex-shrink: 1; height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let row = tree.get(root.children[0]);
        let p = tree.get(row.children[0]);
        let q = tree.get(row.children[1]);

        // 160px of basis must fit into 100px: each shrinks by 30 to 50.
        assert_eq!(p.rect.width, 50.0);
        assert_eq!(q.rect.width, 50.0);
    }

    #[test]
    fn flex_align_items_center_centers_shorter_item_in_row_cross_axis() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"row\"><div class=\"tall\"></div></div>");
        let sheet = placard_css::parse(
            "div.row { display: flex; align-items: center; width: 100px; height: 50px; }
             div.tall { width: 10px; height: 30px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let row = tree.get(root.children[0]);
        let item = tree.get(row.children[0]);

        assert_eq!(item.rect.height, 30.0);
        assert_eq!(item.rect.y, 8.0 + 10.0);
    }

    #[test]
    fn flex_align_items_stretch_grows_item_with_no_explicit_height() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"row\"><div class=\"item\"></div></div>");
        let sheet = placard_css::parse(
            "div.row { display: flex; width: 100px; height: 40px; }
             div.item { width: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let row = tree.get(root.children[0]);
        let item = tree.get(row.children[0]);

        assert_eq!(item.rect.height, 40.0);
    }

    #[test]
    fn flex_wrap_places_overflowing_item_on_a_new_line_with_row_gap() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"wrap\"><div class=\"i1\"></div><div class=\"i2\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.wrap { display: flex; flex-wrap: wrap; width: 100px; row-gap: 5px; }
             div.i1 { width: 60px; height: 10px; }
             div.i2 { width: 60px; height: 20px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let wrap = tree.get(root.children[0]);
        let i1 = tree.get(wrap.children[0]);
        let i2 = tree.get(wrap.children[1]);

        assert_eq!(i1.rect.y, 8.0);
        // Second line starts after the first line's 10px height plus the
        // 5px row-gap.
        assert_eq!(i2.rect.y, 8.0 + 10.0 + 5.0);
    }

    #[test]
    fn flex_column_stacks_with_gap_and_centers_narrower_item() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"col\"><div class=\"narrow\"></div><div class=\"wide\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.col { display: flex; flex-direction: column; align-items: center; width: 100px; gap: 6px; }
             div.narrow { width: 40px; height: 10px; }
             div.wide { width: 80px; height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let col = tree.get(root.children[0]);
        let narrow = tree.get(col.children[0]);
        let wide = tree.get(col.children[1]);

        assert_eq!(narrow.rect.y, 8.0);
        assert_eq!(narrow.rect.x, 8.0 + (100.0 - 40.0) / 2.0);

        assert_eq!(wide.rect.y, 8.0 + 10.0 + 6.0);
        assert_eq!(wide.rect.x, 8.0 + (100.0 - 80.0) / 2.0);
    }

    #[test]
    fn grid_fixed_px_columns_place_cells_side_by_side() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"g\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.g { display: grid; grid-template-columns: 50px 80px; width: 200px; }
             div.a, div.b { height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let g = tree.get(root.children[0]);
        let a = tree.get(g.children[0]);
        let b = tree.get(g.children[1]);

        assert_eq!(a.rect.x, 8.0);
        assert_eq!(a.rect.width, 50.0);
        assert_eq!(b.rect.x, 8.0 + 50.0);
        assert_eq!(b.rect.width, 80.0);
    }

    #[test]
    fn grid_fr_columns_split_remaining_width_by_weight() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"g\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.g { display: grid; grid-template-columns: 1fr 2fr; width: 300px; }
             div.a, div.b { height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let g = tree.get(root.children[0]);
        let a = tree.get(g.children[0]);
        let b = tree.get(g.children[1]);

        assert_eq!(a.rect.width, 100.0);
        assert_eq!(b.rect.width, 200.0);
    }

    #[test]
    fn grid_auto_column_sizes_to_its_widest_cell() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"g\"><div class=\"item\"><div class=\"inner\"></div></div><div class=\"b\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.g { display: grid; grid-template-columns: auto 50px; width: 300px; }
             div.inner { width: 37px; height: 5px; }
             div.b { height: 5px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let g = tree.get(root.children[0]);
        let item = tree.get(g.children[0]);
        let b = tree.get(g.children[1]);

        assert_eq!(item.rect.width, 37.0);
        assert_eq!(b.rect.x, 8.0 + 37.0);
        assert_eq!(b.rect.width, 50.0);
    }

    #[test]
    fn grid_auto_placement_wraps_to_a_second_row() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"g\"><div class=\"a\"></div><div class=\"b\"></div><div class=\"c\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.g { display: grid; grid-template-columns: 50px 50px; width: 200px; }
             div.a, div.b, div.c { height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let g = tree.get(root.children[0]);
        let a = tree.get(g.children[0]);
        let b = tree.get(g.children[1]);
        let c = tree.get(g.children[2]);

        assert_eq!(a.rect.y, 8.0);
        assert_eq!(b.rect.y, 8.0);
        // c wraps past the 2-column limit onto a new row, back at column 0.
        assert_eq!(c.rect.x, 8.0);
        assert_eq!(c.rect.y, 8.0 + 10.0);
    }

    #[test]
    fn grid_row_and_column_gap_apply_independently() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"g\"><div class=\"a\"></div><div class=\"b\"></div><div class=\"c\"></div><div class=\"d\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.g { display: grid; grid-template-columns: 40px 40px; width: 200px; column-gap: 5px; row-gap: 8px; }
             div.a, div.b, div.c, div.d { height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let g = tree.get(root.children[0]);
        let b = tree.get(g.children[1]);
        let c = tree.get(g.children[2]);
        let d = tree.get(g.children[3]);

        assert_eq!(b.rect.x, 8.0 + 40.0 + 5.0);
        assert_eq!(c.rect.y, 8.0 + 10.0 + 8.0);
        assert_eq!(d.rect.x, 8.0 + 40.0 + 5.0);
        assert_eq!(d.rect.y, 8.0 + 10.0 + 8.0);
    }

    #[test]
    fn grid_cells_stretch_to_their_row_height_by_default() {
        let font = test_font();
        let dom = placard_html::parse(
            "<div class=\"g\"><div class=\"tall\"></div><div class=\"short\"></div></div>",
        );
        let sheet = placard_css::parse(
            "div.g { display: grid; grid-template-columns: 50px 50px; width: 200px; }
             div.tall { height: 40px; }
             div.short { height: 10px; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = build(&dom, &styles, &font, 400.0);

        let root = tree.get(tree.root());
        let g = tree.get(root.children[0]);
        let tall = tree.get(g.children[0]);
        let short = tree.get(g.children[1]);

        assert_eq!(tall.rect.height, 40.0);
        assert_eq!(short.rect.height, 40.0);
    }

    #[test]
    fn implicit_body_margin_is_eight_px() {
        let font = test_font();
        let dom = placard_html::parse("<div class=\"box\"></div>");
        let styles = placard_style::compute(&dom, &placard_css::parse(""));

        let tree = build(&dom, &styles, &font, 100.0);
        let div = tree.get(tree.get(tree.root()).children[0]);
        assert_eq!(div.rect.x, 8.0);
        assert_eq!(div.rect.y, 8.0);
    }
}
