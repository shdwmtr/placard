use placard_html::{Dom, NodeData, NodeId};

fn dump(dom: &Dom, styles: &[placard_style::ComputedStyle], id: NodeId, depth: usize) {
    let indent = "  ".repeat(depth);
    let style = &styles[id.index()];
    match dom.data(id) {
        NodeData::Document => {}
        NodeData::Element { tag, .. } => {
            println!(
                "{indent}<{tag}> color={:?} bg={:?} display={:?} font_size={}",
                style.color, style.background_color, style.display, style.font_size
            );
        }
        NodeData::Text(text) => {
            println!(
                "{indent}{text:?} color={:?} font_size={}",
                style.color, style.font_size
            );
        }
    }
    for child in dom.children(id) {
        dump(dom, styles, child, depth + 1);
    }
}

fn run(label: &str, html: &str, css: &str) {
    println!("=== {label} ===");
    println!("html: {html}");
    println!("css:  {css}");
    let dom = placard_html::parse(html);
    let sheet = placard_css::parse(css);
    let styles = placard_style::compute(&dom, &sheet);
    dump(&dom, &styles, dom.root(), 0);
    println!();
}

fn main() {
    run(
        "inheritance: color set on a div is inherited by its text",
        "<div>Hello world</div>",
        "div { color: red; }",
    );

    run(
        "specificity conflict: id beats class beats type, regardless of source order",
        r#"<div id="bar" class="foo">content</div>"#,
        "div { color: green; } .foo { color: blue; } #bar { color: red; }",
    );

    run(
        "background-color is not inherited, only color/font-size/text-align are",
        "<div>text</div>",
        "div { color: purple; background-color: yellow; font-size: 20px; }",
    );
}
