mod colors;
mod parser;
mod types;

pub use types::{
    AttrMatch, Color, Combinator, Declaration, Diagnostic, Rule, Selector, Severity,
    SimpleSelector, Stylesheet, Value,
};

pub fn parse(input: &str) -> Stylesheet {
    parse_with_diagnostics(input).0
}

pub fn parse_declarations(input: &str) -> Vec<Declaration> {
    parse_declarations_with_diagnostics(input).0
}

pub fn parse_with_diagnostics(input: &str) -> (Stylesheet, Vec<Diagnostic>) {
    let mut parser = parser::Parser::new(input);
    let sheet = parser.parse_stylesheet();
    (sheet, parser.into_diagnostics())
}

pub fn parse_declarations_with_diagnostics(input: &str) -> (Vec<Declaration>, Vec<Diagnostic>) {
    let mut parser = parser::Parser::new(input);
    let decls = parser.parse_declarations();
    (decls, parser.into_diagnostics())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_insensitive() {
        let compact = parse("div.foo{color:red;margin:0}");
        let spaced = parse(
            "
            div .foo { }
            ",
        );

        assert_eq!(compact.rules[0].selectors[0].parts.len(), 1);
        assert_eq!(spaced.rules[0].selectors[0].parts.len(), 2);

        let a = parse("div.foo{color:red;margin:0}");
        let b = parse(" div.foo { color : red ; margin : 0 ; } ");
        assert_eq!(a.rules[0].selectors, b.rules[0].selectors);
        assert_eq!(a.rules[0].declarations, b.rules[0].declarations);
    }

    #[test]
    fn comments_are_skipped_everywhere() {
        let sheet = parse(
            "/* leading */ div /* mid-selector */ .foo /* pre-brace */ {
                color: red; /* trailing */
            } /* end */",
        );
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].declarations.len(), 1);
    }

    #[test]
    fn three_and_six_digit_hex_agree() {
        let sheet = parse("a { color: #f00; } b { color: #ff0000; }");
        assert_eq!(
            sheet.rules[0].declarations[0].value,
            sheet.rules[1].declarations[0].value
        );
        assert_eq!(
            sheet.rules[0].declarations[0].value,
            Value::Color(Color::rgb(255, 0, 0))
        );
    }

    #[test]
    fn comma_separated_selector_list_shares_declarations() {
        let sheet = parse("h1, h2, .title { display: block; }");
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors.len(), 3);
        assert_eq!(sheet.rules[0].declarations.len(), 1);
    }

    #[test]
    fn child_vs_descendant_combinator() {
        let child = parse("div > p { }");
        let descendant = parse("div p { }");
        assert_eq!(
            child.rules[0].selectors[0].combinators,
            vec![Combinator::Child]
        );
        assert_eq!(
            descendant.rules[0].selectors[0].combinators,
            vec![Combinator::Descendant]
        );
    }

    #[test]
    fn well_formed_css_has_no_diagnostics() {
        let (_, diags) = parse_with_diagnostics("div.foo { color: red; margin: 0 auto; }");
        assert!(diags.is_empty());
    }

    #[test]
    fn malformed_rule_is_skipped_with_one_diagnostic_and_valid_rules_survive() {
        let (sheet, diags) =
            parse_with_diagnostics("div { color: red; } !!! garbage !!! p { color: blue; }");
        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("garbage"));
    }

    #[test]
    fn malformed_declaration_is_skipped_with_one_diagnostic_and_siblings_survive() {
        let (sheet, diags) =
            parse_with_diagnostics("div { color: red; !!!; background: blue; }");
        assert_eq!(sheet.rules[0].declarations.len(), 2);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("!!!"));
    }

    #[test]
    fn value_display_renders_css_like_text() {
        assert_eq!(Value::Length(12.0).to_string(), "12px");
        assert_eq!(Value::Color(Color::rgb(255, 0, 0)).to_string(), "#ff0000");
        assert_eq!(
            Value::List(vec![Value::Length(1.0), Value::Keyword("auto".into())]).to_string(),
            "1px auto"
        );
    }

    #[test]
    fn selector_display_reconstructs_source_text() {
        let sheet = parse("div.foo > .bar { }");
        assert_eq!(sheet.rules[0].selectors[0].to_string(), "div.foo > .bar");
    }

    #[test]
    fn font_family_keeps_its_original_case() {
        let sheet = parse("div { font-family: MyCustomFont, \"Comic Sans MS\"; }");
        assert_eq!(
            sheet.rules[0].declarations[0].value,
            Value::List(vec![
                Value::Keyword("MyCustomFont".into()),
                Value::Keyword("Comic Sans MS".into()),
            ])
        );
    }

    #[test]
    fn font_shorthand_family_keeps_its_original_case() {
        let sheet = parse("div { font: bold 14px CustomFont; }");
        assert_eq!(
            sheet.rules[0].declarations[0].value,
            Value::List(vec![
                Value::Keyword("bold".into()),
                Value::Length(14.0),
                Value::Keyword("CustomFont".into()),
            ])
        );
    }

    #[test]
    fn other_keyword_properties_are_still_lowercased() {
        let sheet = parse("div { DISPLAY: Block; }");
        assert_eq!(sheet.rules[0].declarations[0].property, "display");
        assert_eq!(
            sheet.rules[0].declarations[0].value,
            Value::Keyword("block".into())
        );
    }
}
