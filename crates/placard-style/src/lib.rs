mod cascade;
mod matching;
mod style;

pub use cascade::{Specificity, compute, lint, specificity};
pub use matching::selector_matches;
pub use placard_css::{Color, Diagnostic, Severity};
pub use style::{
    AlignItems, BorderStyle, BoxSizing, ComputedStyle, Dimension, Display, FlexDirection, FlexWrap,
    FontFamily, FontStyle, FontWeight, JustifyContent, LineHeight, Position, Side, TextAlign,
    TrackSize,
};

#[cfg(test)]
mod tests {
    use super::*;
    use placard_css::Color;

    #[test]
    fn specificity_ordering() {
        let type_only = placard_css::parse("div { }").rules[0].selectors[0].clone();
        let class_only = placard_css::parse(".foo { }").rules[0].selectors[0].clone();
        let id_only = placard_css::parse("#bar { }").rules[0].selectors[0].clone();
        let type_and_class = placard_css::parse("div.foo { }").rules[0].selectors[0].clone();

        assert!(specificity(&class_only) > specificity(&type_only));
        assert!(specificity(&id_only) > specificity(&class_only));
        assert!(specificity(&type_and_class) > specificity(&type_only));
        assert_eq!(specificity(&type_only), (0, 0, 1));
        assert_eq!(specificity(&class_only), (0, 1, 0));
        assert_eq!(specificity(&id_only), (1, 0, 0));
    }

    #[test]
    fn later_source_order_wins_at_equal_specificity() {
        let dom = placard_html::parse("<div class=\"a\"></div>");
        let sheet = placard_css::parse(".a { color: red; } .a { color: blue; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[div.index()].color, Color::rgb(0, 0, 255));
    }

    #[test]
    fn higher_specificity_wins_regardless_of_order() {
        let dom = placard_html::parse("<div id=\"bar\" class=\"foo\"></div>");
        let sheet =
            placard_css::parse("#bar { color: red; } .foo { color: blue; } div { color: green; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[div.index()].color, Color::rgb(255, 0, 0));
    }

    #[test]
    fn border_radius_shorthand_expands_per_corner() {
        let dom = placard_html::parse("<div class=\"a\"></div><div class=\"b\"></div>");
        let sheet = placard_css::parse(
            "div.a { border-radius: 5px 0 0 5px; }
             div.b { border-radius: 4px 8px; }",
        );
        let styles = compute(&dom, &sheet);
        let a = dom.first_child(dom.root()).unwrap();
        let b = dom.children(dom.root()).nth(1).unwrap();

        assert_eq!(styles[a.index()].border_radius, [5.0, 0.0, 0.0, 5.0]);
        assert_eq!(styles[b.index()].border_radius, [4.0, 8.0, 4.0, 8.0]);
    }

    #[test]
    fn inline_style_attribute_applies_declarations() {
        let dom = placard_html::parse(
            "<span style=\"background:#555;color:#fff;padding:2px 6px\">text</span>",
        );
        let styles = compute(&dom, &placard_css::parse(""));
        let span = dom.first_child(dom.root()).unwrap();
        assert_eq!(
            styles[span.index()].background_color,
            Color::rgb(85, 85, 85)
        );
        assert_eq!(styles[span.index()].color, Color::rgb(255, 255, 255));
        assert_eq!(styles[span.index()].padding, [2.0, 6.0, 2.0, 6.0]);
    }

    #[test]
    fn inline_style_attribute_overrides_stylesheet_regardless_of_specificity() {
        let dom = placard_html::parse("<span id=\"x\" style=\"color:blue\">text</span>");
        let sheet = placard_css::parse("#x { color: red; }");
        let styles = compute(&dom, &sheet);
        let span = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[span.index()].color, Color::rgb(0, 0, 255));
    }

    #[test]
    fn flex_shorthand_single_number_defaults_shrink_one_basis_zero() {
        let dom = placard_html::parse("<div class=\"a\"></div>");
        let sheet = placard_css::parse("div.a { display: flex; } div.a { flex: 2; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[div.index()].display, Display::Flex);
        assert_eq!(styles[div.index()].flex_grow, 2.0);
        assert_eq!(styles[div.index()].flex_shrink, 1.0);
        assert_eq!(
            styles[div.index()].flex_basis,
            Some(Dimension::Percent(0.0))
        );
    }

    #[test]
    fn flex_shorthand_three_values_sets_grow_shrink_and_px_basis() {
        let dom = placard_html::parse("<div class=\"a\"></div>");
        let sheet = placard_css::parse("div.a { flex: 2 1 50px; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[div.index()].flex_grow, 2.0);
        assert_eq!(styles[div.index()].flex_shrink, 1.0);
        assert_eq!(styles[div.index()].flex_basis, Some(Dimension::Px(50.0)));
    }

    #[test]
    fn gap_shorthand_with_two_values_sets_row_and_column_independently() {
        let dom = placard_html::parse("<div class=\"a\"></div>");
        let sheet = placard_css::parse("div.a { gap: 4px 8px; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[div.index()].row_gap, 4.0);
        assert_eq!(styles[div.index()].column_gap, 8.0);
    }

    #[test]
    fn grid_template_columns_parses_mixed_px_fr_and_auto_tracks() {
        let dom = placard_html::parse("<div class=\"a\"></div>");
        let sheet =
            placard_css::parse("div.a { display: grid; grid-template-columns: 100px 1fr auto; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();

        assert_eq!(styles[div.index()].display, Display::Grid);
        assert_eq!(
            styles[div.index()].grid_template_columns,
            vec![TrackSize::Px(100.0), TrackSize::Fr(1.0), TrackSize::Auto,]
        );
    }

    #[test]
    fn color_is_inherited_but_background_is_not() {
        let dom = placard_html::parse("<div>text</div>");
        let sheet = placard_css::parse("div { color: red; background-color: blue; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();
        let text = dom.first_child(div).unwrap();
        assert_eq!(styles[text.index()].color, Color::rgb(255, 0, 0));
        assert_eq!(
            styles[text.index()].background_color,
            Color::rgba(0, 0, 0, 0)
        );
    }

    #[test]
    fn lint_is_clean_for_recognized_properties() {
        let dom = placard_html::parse("<div></div>");
        let sheet = placard_css::parse("div { color: red; margin: 0 auto; }");
        assert!(lint(&dom, &sheet).is_empty());
    }

    #[test]
    fn lint_flags_unrecognized_property_once_per_rule_not_per_match() {
        let dom = placard_html::parse("<div></div><div></div><div></div>");
        let sheet = placard_css::parse("div { text-decoration: underline; }");
        let diags = lint(&dom, &sheet);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("text-decoration"));
        assert!(diags[0].message.contains("div"));
    }

    #[test]
    fn lint_flags_unrecognized_property_in_inline_style() {
        let dom = placard_html::parse("<div style=\"cursor: pointer\"></div>");
        let diags = lint(&dom, &placard_css::parse(""));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("cursor"));
        assert!(diags[0].message.contains("div"));
    }

    #[test]
    fn empty_font_family_entry_is_ignored_rather_than_named_empty_string() {
        let dom = placard_html::parse("<div style=\"font-family: '', sans-serif\">hi</div>");
        let styles = compute(&dom, &placard_css::parse(""));
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[div.index()].font_family, vec![FontFamily::SansSerif]);
    }

    #[test]
    fn font_family_left_inherited_when_declared_value_is_only_an_empty_string() {
        let dom = placard_html::parse(
            "<div style=\"font-family: monospace\"><span style=\"font-family: ''\">hi</span></div>",
        );
        let styles = compute(&dom, &placard_css::parse(""));
        let div = dom.first_child(dom.root()).unwrap();
        let span = dom.first_child(div).unwrap();
        assert_eq!(styles[span.index()].font_family, vec![FontFamily::Monospace]);
    }

    #[test]
    fn letter_spacing_accepts_px_and_negative_em_values() {
        let dom = placard_html::parse("<div class=\"a\"></div><div class=\"b\"></div>");
        let sheet = placard_css::parse(
            "div.a { letter-spacing: 2px; }
             div.b { font-size: 20px; letter-spacing: -0.05em; }",
        );
        let styles = compute(&dom, &sheet);
        let a = dom.first_child(dom.root()).unwrap();
        let b = dom.children(dom.root()).nth(1).unwrap();

        assert_eq!(styles[a.index()].letter_spacing, 2.0);
        assert_eq!(styles[b.index()].letter_spacing, -1.0);
    }

    #[test]
    fn letter_spacing_normal_keyword_resets_to_zero() {
        let dom = placard_html::parse("<div style=\"letter-spacing: normal\"></div>");
        let styles = compute(&dom, &placard_css::parse(""));
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(styles[div.index()].letter_spacing, 0.0);
    }

    #[test]
    fn letter_spacing_is_inherited() {
        let dom = placard_html::parse("<div>text</div>");
        let sheet = placard_css::parse("div { letter-spacing: 3px; }");
        let styles = compute(&dom, &sheet);
        let div = dom.first_child(dom.root()).unwrap();
        let text = dom.first_child(div).unwrap();
        assert_eq!(styles[text.index()].letter_spacing, 3.0);
    }

    #[test]
    fn lint_recognizes_letter_spacing() {
        let dom = placard_html::parse("<div></div>");
        let sheet = placard_css::parse("div { letter-spacing: 1px; }");
        assert!(lint(&dom, &sheet).is_empty());
    }
}
