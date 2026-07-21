use crate::matching::selector_matches;
use crate::style::{
    AlignItems, BorderStyle, BoxSizing, ComputedStyle, Dimension, Display, FlexDirection, FlexWrap,
    FontFamily, FontStyle, FontWeight, JustifyContent, LineHeight, Position, TextAlign, TrackSize,
};
use placard_css::{Color, Declaration, Diagnostic, Selector, Stylesheet, Value};
use placard_html::{Dom, NodeId};

pub type Specificity = (u32, u32, u32);

const ROOT_FONT_SIZE: f32 = 16.0;

pub fn lint(dom: &Dom, stylesheet: &Stylesheet) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for rule in &stylesheet.rules {
        let selector_text = rule
            .selectors
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        for decl in &rule.declarations {
            let mut scratch = ComputedStyle::initial();
            if !apply_declaration(&mut scratch, decl) {
                diagnostics.push(Diagnostic::warning(format!(
                    "unrecognized CSS property `{}` in `{selector_text}`",
                    decl.property
                )));
            }
        }
    }

    lint_inline_styles(dom, dom.root(), &mut diagnostics);
    diagnostics
}

fn lint_inline_styles(dom: &Dom, node: NodeId, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(inline) = dom.attr(node, "style") {
        let (decls, parse_diagnostics) = placard_css::parse_declarations_with_diagnostics(inline);
        diagnostics.extend(parse_diagnostics);
        let tag = dom.tag(node).unwrap_or("");
        for decl in &decls {
            let mut scratch = ComputedStyle::initial();
            if !apply_declaration(&mut scratch, decl) {
                diagnostics.push(Diagnostic::warning(format!(
                    "unrecognized CSS property `{}` in inline style on <{tag}>",
                    decl.property
                )));
            }
        }
    }
    for child in dom.children(node) {
        lint_inline_styles(dom, child, diagnostics);
    }
}

pub fn specificity(selector: &Selector) -> Specificity {
    let mut ids = 0;
    let mut classes = 0;
    let mut types = 0;
    for part in &selector.parts {
        if part.id.is_some() {
            ids += 1;
        }
        classes += (part.classes.len() + part.attrs.len()) as u32;
        if part.tag.is_some() {
            types += 1;
        }
    }
    (ids, classes, types)
}

pub fn compute(dom: &Dom, stylesheet: &Stylesheet) -> Vec<ComputedStyle> {
    let mut styles: Vec<Option<ComputedStyle>> = vec![None; dom.node_count()];
    let root = dom.root();
    let root_style = ComputedStyle::initial();
    styles[root.index()] = Some(root_style.clone());

    compute_children(dom, stylesheet, root, &root_style, &mut styles);

    styles
        .into_iter()
        .map(|s| s.unwrap_or_else(ComputedStyle::initial))
        .collect()
}

fn compute_children(
    dom: &Dom,
    stylesheet: &Stylesheet,
    node: NodeId,
    parent_style: &ComputedStyle,
    styles: &mut [Option<ComputedStyle>],
) {
    for child in dom.children(node) {
        let style = if dom.tag(child).is_some() {
            compute_element_style(dom, stylesheet, child, parent_style)
        } else {
            ComputedStyle::inherited_from(parent_style)
        };
        styles[child.index()] = Some(style.clone());
        compute_children(dom, stylesheet, child, &style, styles);
    }
}

fn compute_element_style(
    dom: &Dom,
    stylesheet: &Stylesheet,
    node: NodeId,
    parent_style: &ComputedStyle,
) -> ComputedStyle {
    let mut matched: Vec<(Specificity, usize, &Declaration)> = Vec::new();

    for (order, rule) in stylesheet.rules.iter().enumerate() {
        let best_specificity = rule
            .selectors
            .iter()
            .filter(|sel| selector_matches(dom, node, sel))
            .map(specificity)
            .max();

        if let Some(spec) = best_specificity {
            for decl in &rule.declarations {
                matched.push((spec, order, decl));
            }
        }
    }

    matched.sort_by_key(|(spec, order, _)| (*spec, *order));

    let mut style = ComputedStyle::inherited_from(parent_style);
    let tag = dom.tag(node).unwrap_or("");
    style.display = default_display(tag);
    apply_user_agent_defaults(&mut style, tag);
    for (_, _, decl) in matched {
        apply_declaration(&mut style, decl);
    }
    if let Some(inline) = dom.attr(node, "style") {
        for decl in placard_css::parse_declarations(inline) {
            apply_declaration(&mut style, &decl);
        }
    }
    style
}

const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

fn as_slice(value: &Value) -> Vec<&Value> {
    match value {
        Value::List(items) => items.iter().collect(),
        other => vec![other],
    }
}

fn expand_trbl<T: Copy>(
    values: &[&Value],
    resolve: impl Fn(&Value) -> Option<T>,
) -> Option<[T; 4]> {
    if values.is_empty() || values.len() > 4 {
        return None;
    }
    let mut resolved = Vec::with_capacity(values.len());
    for v in values {
        resolved.push(resolve(v)?);
    }
    Some(match resolved.len() {
        1 => [resolved[0]; 4],
        2 => [resolved[0], resolved[1], resolved[0], resolved[1]],
        3 => [resolved[0], resolved[1], resolved[2], resolved[1]],
        _ => [resolved[0], resolved[1], resolved[2], resolved[3]],
    })
}

fn resolve_length_component(v: &Value, font_size: f32) -> Option<f32> {
    match v {
        Value::Length(l) => Some(*l),
        Value::Em(e) => Some(e * font_size),
        Value::Rem(r) => Some(r * ROOT_FONT_SIZE),
        _ => None,
    }
}

fn resolve_margin_component(v: &Value, font_size: f32) -> Option<Option<f32>> {
    match v {
        Value::Keyword(k) if k == "auto" => Some(None),
        _ => resolve_length_component(v, font_size).map(Some),
    }
}

/// Shared by `top`/`right`/`bottom`/`left` and `flex-basis`, all of which
/// accept either a length/percentage or the keyword `auto`.
fn auto_or_dimension(v: &Value, font_size: f32) -> Option<Option<Dimension>> {
    match v {
        Value::Keyword(k) if k == "auto" => Some(None),
        _ => dimension_value(v, font_size).map(Some),
    }
}

fn dimension_value(v: &Value, font_size: f32) -> Option<Dimension> {
    match v {
        Value::Length(l) => Some(Dimension::Px(*l)),
        Value::Percent(p) => Some(Dimension::Percent(*p)),
        Value::Em(e) => Some(Dimension::Px(e * font_size)),
        Value::Rem(r) => Some(Dimension::Px(r * ROOT_FONT_SIZE)),
        _ => None,
    }
}

fn track_size_value(v: &Value, font_size: f32) -> Option<TrackSize> {
    match v {
        Value::Keyword(k) if k == "auto" => Some(TrackSize::Auto),
        Value::Fr(f) => Some(TrackSize::Fr(*f)),
        Value::Percent(p) => Some(TrackSize::Percent(*p)),
        _ => resolve_length_component(v, font_size).map(TrackSize::Px),
    }
}

fn track_list(value: &Value, font_size: f32) -> Vec<TrackSize> {
    as_slice(value)
        .iter()
        .filter_map(|v| track_size_value(v, font_size))
        .collect()
}

fn font_size_value(v: &Value, basis_font_size: f32) -> Option<f32> {
    match v {
        Value::Length(l) => Some(*l),
        Value::Em(e) => Some(e * basis_font_size),
        Value::Rem(r) => Some(r * ROOT_FONT_SIZE),
        Value::Percent(p) => Some(basis_font_size * p / 100.0),
        _ => None,
    }
}

fn color_value(v: &Value) -> Option<Color> {
    match v {
        Value::Color(c) => Some(*c),
        _ => None,
    }
}

fn keyword_value(v: &Value) -> Option<&str> {
    match v {
        Value::Keyword(k) => Some(k.as_str()),
        _ => None,
    }
}

fn font_weight_value(v: &Value) -> Option<FontWeight> {
    match v {
        Value::Keyword(k) if k == "bold" => Some(FontWeight::Bold),
        Value::Keyword(k) if k == "normal" => Some(FontWeight::Normal),
        Value::Length(n) => Some(if *n >= 600.0 {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        }),
        _ => None,
    }
}

fn font_style_value(v: &Value) -> Option<FontStyle> {
    match v {
        Value::Keyword(k) if k == "italic" || k == "oblique" => Some(FontStyle::Italic),
        Value::Keyword(k) if k == "normal" => Some(FontStyle::Normal),
        _ => None,
    }
}

/// `k` keeps whatever case the author wrote a font name in (see
/// `placard_css::Parser`'s `preserve_case` handling for `font`/`font-family`)
/// -- generic keywords are still recognized case-insensitively here, since
/// those really are case-insensitive CSS keywords, but a name that isn't one
/// of them is kept verbatim rather than folded to lowercase, since it still
/// needs to reach the font matcher (itself case-insensitive) looking like
/// what the author actually typed.
fn font_family_value(v: &Value) -> Option<FontFamily> {
    match v {
        Value::Keyword(k) if k.is_empty() => None,
        Value::Keyword(k) => Some(match k.to_ascii_lowercase().as_str() {
            "serif" => FontFamily::Serif,
            "monospace" | "mono" => FontFamily::Monospace,
            "sans-serif" | "sans" | "arial" | "helvetica" | "system-ui" => FontFamily::SansSerif,
            _ => FontFamily::Named(k.clone()),
        }),
        _ => None,
    }
}

fn line_height_value(v: &Value, font_size: f32) -> Option<LineHeight> {
    match v {
        Value::Keyword(k) if k == "normal" => Some(LineHeight::Normal),
        Value::Length(l) if *l <= 10.0 => Some(LineHeight::Number(*l)),
        Value::Length(l) => Some(LineHeight::Px(*l)),
        Value::Percent(p) => Some(LineHeight::Number(p / 100.0)),
        Value::Em(e) => Some(LineHeight::Px(e * font_size)),
        Value::Rem(r) => Some(LineHeight::Px(r * ROOT_FONT_SIZE)),
        _ => None,
    }
}

fn letter_spacing_value(v: &Value, font_size: f32) -> Option<f32> {
    match v {
        Value::Keyword(k) if k == "normal" => Some(0.0),
        _ => resolve_length_component(v, font_size),
    }
}

fn apply_border_shorthand(style: &mut ComputedStyle, comps: &[&Value]) {
    let mut width = None;
    let mut border_style = None;
    let mut color = None;

    for v in comps {
        if width.is_none() {
            if let Some(w) = resolve_length_component(v, style.font_size) {
                width = Some(w);
                continue;
            }
        }
        if let Some(c) = color_value(v) {
            color = Some(c);
            continue;
        }
        if let Some(k) = keyword_value(v) {
            border_style = Some(border_style_from_keyword(k));
        }
    }

    if let Some(w) = width {
        style.border_width = [w; 4];
    }
    if let Some(s) = border_style {
        style.border_style = [s; 4];
    }
    if let Some(c) = color {
        style.border_color = [c; 4];
    }
}

/// The `flex-grow`/`flex-shrink` slots and a `px` length both parse to the
/// same untagged `Value::Length`, so a bare `flex: <n> <px>` two-value form
/// (basis in px, shrink defaulting to 1) is indistinguishable here from
/// `flex: <n> <n>` (grow and shrink, basis defaulting to 0) -- the first two
/// `Length`s found are always read as grow/shrink. The unambiguous 3-value
/// form (`flex: <grow> <shrink> <basis>`), `flex: none`, and `flex: auto`
/// all resolve correctly; only that one two-value spelling doesn't.
fn apply_flex_shorthand(style: &mut ComputedStyle, comps: &[&Value]) {
    if comps.len() == 1 {
        if let Value::Keyword(k) = comps[0] {
            match k.as_str() {
                "none" => {
                    style.flex_grow = 0.0;
                    style.flex_shrink = 0.0;
                    style.flex_basis = None;
                    return;
                }
                "auto" => {
                    style.flex_grow = 1.0;
                    style.flex_shrink = 1.0;
                    style.flex_basis = None;
                    return;
                }
                _ => {}
            }
        }
    }

    let mut grow = None;
    let mut shrink = None;
    let mut basis: Option<Option<Dimension>> = None;

    for v in comps {
        if grow.is_none() {
            if let Value::Length(n) = v {
                grow = Some(*n);
                continue;
            }
        } else if shrink.is_none() {
            if let Value::Length(n) = v {
                shrink = Some(*n);
                continue;
            }
        }
        if let Some(b) = auto_or_dimension(v, style.font_size) {
            basis = Some(b);
        }
    }

    if let Some(g) = grow {
        style.flex_grow = g;
        style.flex_shrink = shrink.unwrap_or(1.0);
        style.flex_basis = basis.unwrap_or(Some(Dimension::Percent(0.0)));
    }
}

fn apply_gap_shorthand(style: &mut ComputedStyle, comps: &[&Value]) {
    let vals: Vec<f32> = comps
        .iter()
        .filter_map(|v| resolve_length_component(v, style.font_size))
        .collect();
    match vals.len() {
        1 => {
            style.row_gap = vals[0];
            style.column_gap = vals[0];
        }
        2 => {
            style.row_gap = vals[0];
            style.column_gap = vals[1];
        }
        _ => {}
    }
}

fn apply_font_shorthand(style: &mut ComputedStyle, comps: &[&Value]) {
    let mut have_size = false;
    let mut families = Vec::new();

    for v in comps {
        if let Value::Keyword(k) = v {
            match k.to_ascii_lowercase().as_str() {
                "italic" | "oblique" => {
                    style.font_style = FontStyle::Italic;
                    continue;
                }
                "bold" => {
                    style.font_weight = FontWeight::Bold;
                    continue;
                }
                "normal" => continue,
                _ => {}
            }
            if let Some(ff) = font_family_value(v) {
                families.push(ff);
            }
            continue;
        }

        if let Some(sz) = font_size_value(v, style.font_size) {
            if !have_size {
                style.font_size = sz;
                have_size = true;
            } else {
                style.line_height = LineHeight::Px(sz);
            }
        }
    }

    if !families.is_empty() {
        style.font_family = families;
    }
}

/// Applies `decl` to `style`, returning whether the property name was
/// recognized at all -- regardless of whether the value ended up in a shape
/// this renderer knows how to resolve. Callers that only care about the
/// visual result can ignore the return value; `lint` uses it to flag
/// declarations placard doesn't understand at all.
fn apply_declaration(style: &mut ComputedStyle, decl: &Declaration) -> bool {
    let prop = decl.property.as_str();
    let comps = as_slice(&decl.value);

    match prop {
        "margin" => {
            if let Some(sides) =
                expand_trbl(&comps, |v| resolve_margin_component(v, style.font_size))
            {
                style.margin = sides;
            }
            return true;
        }
        "padding" => {
            if let Some(sides) =
                expand_trbl(&comps, |v| resolve_length_component(v, style.font_size))
            {
                style.padding = sides;
            }
            return true;
        }
        "border-width" => {
            if let Some(sides) =
                expand_trbl(&comps, |v| resolve_length_component(v, style.font_size))
            {
                style.border_width = sides;
            }
            return true;
        }
        "border-color" => {
            if let Some(sides) = expand_trbl(&comps, color_value) {
                style.border_color = sides;
            }
            return true;
        }
        "border-style" => {
            if let Some(sides) =
                expand_trbl(&comps, |v| keyword_value(v).map(border_style_from_keyword))
            {
                style.border_style = sides;
            }
            return true;
        }
        "border" => {
            apply_border_shorthand(style, &comps);
            return true;
        }
        "background" => {
            if let Some(c) = comps.iter().find_map(|v| color_value(v)) {
                style.background_color = c;
            }
            return true;
        }
        "font" => {
            apply_font_shorthand(style, &comps);
            return true;
        }
        "font-weight" => {
            if let Some(fw) = comps.first().and_then(|v| font_weight_value(v)) {
                style.font_weight = fw;
            }
            return true;
        }
        "font-style" => {
            if let Some(fs) = comps.first().and_then(|v| font_style_value(v)) {
                style.font_style = fs;
            }
            return true;
        }
        "font-family" => {
            let families: Vec<FontFamily> =
                comps.iter().filter_map(|v| font_family_value(v)).collect();
            if !families.is_empty() {
                style.font_family = families;
            }
            return true;
        }
        "line-height" => {
            if let Some(lh) = comps
                .first()
                .and_then(|v| line_height_value(v, style.font_size))
            {
                style.line_height = lh;
            }
            return true;
        }
        "letter-spacing" => {
            if let Some(ls) = comps
                .first()
                .and_then(|v| letter_spacing_value(v, style.font_size))
            {
                style.letter_spacing = ls;
            }
            return true;
        }
        "box-sizing" => {
            if let Some(k) = comps.first().and_then(|v| keyword_value(v)) {
                style.box_sizing = if k == "border-box" {
                    BoxSizing::BorderBox
                } else {
                    BoxSizing::ContentBox
                };
            }
            return true;
        }
        "flex" => {
            apply_flex_shorthand(style, &comps);
            return true;
        }
        "gap" => {
            apply_gap_shorthand(style, &comps);
            return true;
        }
        "grid-template-columns" => {
            style.grid_template_columns = track_list(&decl.value, style.font_size);
            return true;
        }
        "grid-template-rows" => {
            style.grid_template_rows = track_list(&decl.value, style.font_size);
            return true;
        }
        _ => {}
    }

    if let Some(side_name) = prop.strip_prefix("margin-") {
        if let Some(idx) = SIDES.iter().position(|s| *s == side_name) {
            if let Some(v) = resolve_margin_component(&decl.value, style.font_size) {
                style.margin[idx] = v;
            }
            return true;
        }
        return false;
    }
    if let Some(side_name) = prop.strip_prefix("padding-") {
        if let Some(idx) = SIDES.iter().position(|s| *s == side_name) {
            if let Some(v) = resolve_length_component(&decl.value, style.font_size) {
                style.padding[idx] = v;
            }
            return true;
        }
        return false;
    }
    if let Some(idx) = SIDES.iter().position(|s| *s == prop) {
        if let Some(v) = auto_or_dimension(&decl.value, style.font_size) {
            style.inset[idx] = v;
        }
        return true;
    }
    if let Some(rest) = prop.strip_prefix("border-") {
        for (idx, side_name) in SIDES.iter().enumerate() {
            if let Some(subprop) = rest
                .strip_prefix(side_name)
                .and_then(|s| s.strip_prefix('-'))
            {
                match subprop {
                    "width" => {
                        if let Some(v) = resolve_length_component(&decl.value, style.font_size) {
                            style.border_width[idx] = v;
                        }
                    }
                    "color" => {
                        if let Some(c) = color_value(&decl.value) {
                            style.border_color[idx] = c;
                        }
                    }
                    "style" => {
                        if let Some(k) = keyword_value(&decl.value) {
                            style.border_style[idx] = border_style_from_keyword(k);
                        }
                    }
                    _ => return false,
                }
                return true;
            }
        }
        if rest == "radius" {
            if let Some(v) = expand_trbl(&comps, |v| resolve_length_component(v, style.font_size)) {
                style.border_radius = v;
            }
            return true;
        }
        return false;
    }

    match prop {
        "display" => {
            if let Value::Keyword(k) = &decl.value {
                style.display = display_from_keyword(k);
            }
            true
        }
        "color" => {
            if let Value::Color(c) = &decl.value {
                style.color = *c;
            }
            true
        }
        "background-color" => {
            if let Value::Color(c) = &decl.value {
                style.background_color = *c;
            }
            true
        }
        "font-size" => {
            if let Some(v) = font_size_value(&decl.value, style.font_size) {
                style.font_size = v.max(0.0);
            }
            true
        }
        "width" => {
            style.width = dimension_value(&decl.value, style.font_size);
            true
        }
        "height" => {
            style.height = dimension_value(&decl.value, style.font_size);
            true
        }
        "text-align" => {
            if let Value::Keyword(k) = &decl.value {
                style.text_align = text_align_from_keyword(k);
            }
            true
        }
        "position" => {
            if let Value::Keyword(k) = &decl.value {
                style.position = position_from_keyword(k);
            }
            true
        }
        "z-index" => {
            if let Some(v) = resolve_length_component(&decl.value, style.font_size) {
                style.z_index = Some(v as i32);
            }
            true
        }
        "flex-direction" => {
            if let Value::Keyword(k) = &decl.value {
                style.flex_direction = flex_direction_from_keyword(k);
            }
            true
        }
        "flex-wrap" => {
            if let Value::Keyword(k) = &decl.value {
                style.flex_wrap = flex_wrap_from_keyword(k);
            }
            true
        }
        "justify-content" => {
            if let Value::Keyword(k) = &decl.value {
                style.justify_content = justify_content_from_keyword(k);
            }
            true
        }
        "align-items" => {
            if let Value::Keyword(k) = &decl.value {
                style.align_items = align_items_from_keyword(k);
            }
            true
        }
        "align-self" => {
            if let Value::Keyword(k) = &decl.value {
                style.align_self = if k == "auto" {
                    None
                } else {
                    Some(align_items_from_keyword(k))
                };
            }
            true
        }
        "flex-grow" => {
            if let Some(n) = resolve_length_component(&decl.value, style.font_size) {
                style.flex_grow = n.max(0.0);
            }
            true
        }
        "flex-shrink" => {
            if let Some(n) = resolve_length_component(&decl.value, style.font_size) {
                style.flex_shrink = n.max(0.0);
            }
            true
        }
        "flex-basis" => {
            if let Some(b) = auto_or_dimension(&decl.value, style.font_size) {
                style.flex_basis = b;
            }
            true
        }
        "row-gap" => {
            if let Some(v) = resolve_length_component(&decl.value, style.font_size) {
                style.row_gap = v;
            }
            true
        }
        "column-gap" => {
            if let Some(v) = resolve_length_component(&decl.value, style.font_size) {
                style.column_gap = v;
            }
            true
        }
        _ => false,
    }
}

fn default_display(tag: &str) -> Display {
    match tag {
        "html" | "body" | "div" | "p" | "ul" | "ol" | "li" | "h1" | "h2" | "h3" | "h4" | "h5"
        | "h6" | "header" | "footer" | "nav" | "section" | "article" | "aside" | "main"
        | "figure" | "figcaption" | "blockquote" | "pre" | "address" | "dl" | "dt" | "dd"
        | "details" | "summary" | "dialog" | "hr" | "form" | "fieldset" | "legend" | "table"
        | "tr" | "thead" | "tbody" | "tfoot" | "caption" => Display::Block,

        "style" | "script" | "head" | "title" | "meta" | "link" => Display::None,
        _ => Display::Inline,
    }
}

fn apply_user_agent_defaults(style: &mut ComputedStyle, tag: &str) {
    match tag {
        "body" => style.margin = [Some(8.0); 4],
        "h1" => apply_heading_defaults(style, 2.0, 0.67),
        "h2" => apply_heading_defaults(style, 1.5, 0.83),
        "h3" => apply_heading_defaults(style, 1.17, 1.0),
        "h4" => apply_heading_defaults(style, 1.0, 1.33),
        "h5" => apply_heading_defaults(style, 0.83, 1.67),
        "h6" => apply_heading_defaults(style, 0.67, 2.33),
        "p" | "dl" | "hr" => {
            let m = Some(style.font_size);
            style.margin = [m, Some(0.0), m, Some(0.0)];
        }
        "pre" => {
            let m = Some(style.font_size);
            style.margin = [m, Some(0.0), m, Some(0.0)];
            style.font_family = vec![FontFamily::Monospace];
        }
        "blockquote" | "figure" => {
            let m = Some(style.font_size);
            style.margin = [m, Some(40.0), m, Some(40.0)];
        }
        "ul" | "ol" => {
            let m = Some(style.font_size);
            style.margin = [m, Some(0.0), m, Some(0.0)];
            style.padding[3] = 40.0;
        }
        "b" | "strong" => style.font_weight = FontWeight::Bold,
        "i" | "em" | "cite" | "dfn" | "var" | "address" => style.font_style = FontStyle::Italic,
        "small" | "sub" | "sup" => style.font_size *= 0.83,
        "code" | "kbd" | "samp" => style.font_family = vec![FontFamily::Monospace],
        "a" => style.color = placard_css::Color::rgb(0, 0, 238),
        _ => {}
    }
}

fn apply_heading_defaults(style: &mut ComputedStyle, size_em: f32, margin_em: f32) {
    style.font_size *= size_em;
    style.font_weight = FontWeight::Bold;
    let m = Some(margin_em * style.font_size);
    style.margin = [m, Some(0.0), m, Some(0.0)];
}

fn display_from_keyword(k: &str) -> Display {
    match k {
        "none" => Display::None,
        "inline" => Display::Inline,
        "flex" => Display::Flex,
        "grid" => Display::Grid,
        _ => Display::Block,
    }
}

fn border_style_from_keyword(k: &str) -> BorderStyle {
    match k {
        "solid" => BorderStyle::Solid,
        _ => BorderStyle::None,
    }
}

fn text_align_from_keyword(k: &str) -> TextAlign {
    match k {
        "right" => TextAlign::Right,
        "center" => TextAlign::Center,
        _ => TextAlign::Left,
    }
}

fn position_from_keyword(k: &str) -> Position {
    match k {
        "relative" => Position::Relative,
        "absolute" => Position::Absolute,
        "fixed" => Position::Fixed,
        _ => Position::Static,
    }
}

fn flex_direction_from_keyword(k: &str) -> FlexDirection {
    match k {
        "column" => FlexDirection::Column,
        _ => FlexDirection::Row,
    }
}

fn flex_wrap_from_keyword(k: &str) -> FlexWrap {
    match k {
        "wrap" => FlexWrap::Wrap,
        _ => FlexWrap::NoWrap,
    }
}

fn justify_content_from_keyword(k: &str) -> JustifyContent {
    match k {
        "flex-end" | "end" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "space-between" => JustifyContent::SpaceBetween,
        "space-around" => JustifyContent::SpaceAround,
        "space-evenly" => JustifyContent::SpaceEvenly,
        _ => JustifyContent::FlexStart,
    }
}

fn align_items_from_keyword(k: &str) -> AlignItems {
    match k {
        "flex-end" | "end" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "flex-start" | "start" => AlignItems::FlexStart,
        _ => AlignItems::Stretch,
    }
}
