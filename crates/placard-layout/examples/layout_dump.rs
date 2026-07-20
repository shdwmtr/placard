use placard_font::{Font, FontSet};
use placard_layout::{BoxKind, LayoutNodeId, LayoutTree};

fn dump(tree: &LayoutTree, id: LayoutNodeId, depth: usize) {
    let indent = "  ".repeat(depth);
    let node = tree.get(id);
    let r = node.rect;
    match &node.kind {
        BoxKind::Block => {
            println!(
                "{indent}Block  x={:.1} y={:.1} w={:.1} h={:.1}  margin={:?} padding={:?} border_width={:?}",
                r.x,
                r.y,
                r.width,
                r.height,
                node.style.margin,
                node.style.padding,
                node.style.border_width
            );
        }
        BoxKind::Text { content } => {
            println!(
                "{indent}Text {content:?}  x={:.1} y={:.1} w={:.1} h={:.1}",
                r.x, r.y, r.width, r.height
            );
        }
        BoxKind::InlineBackground => {
            println!(
                "{indent}InlineBackground  x={:.1} y={:.1} w={:.1} h={:.1}",
                r.x, r.y, r.width, r.height
            );
        }
    }
    for &child in tree.children(id) {
        dump(tree, child, depth + 1);
    }
}

fn main() {
    let font_data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
        .expect("failed to read font");
    let font = FontSet::new(Font::parse(&font_data).expect("failed to parse font"));

    let html = r#"
        <div class="outer">
            <div class="inner">
                <p class="text">The quick brown fox jumps over the lazy dog and keeps running past the hill.</p>
            </div>
        </div>
    "#;

    let css = r#"
        div.outer {
            margin-top: 10px;
            margin-left: 20px;
            border-top-width: 2px;
            border-right-width: 2px;
            border-bottom-width: 2px;
            border-left-width: 2px;
            border-top-style: solid;
            border-right-style: solid;
            border-bottom-style: solid;
            border-left-style: solid;
        }
        div.inner {
            padding-top: 8px;
            padding-right: 8px;
            padding-bottom: 8px;
            padding-left: 8px;
        }
        p.text {
            font-size: 16px;
        }
    "#;

    let dom = placard_html::parse(html);
    let sheet = placard_css::parse(css);
    let styles = placard_style::compute(&dom, &sheet);
    let viewport_width = 220.0;
    let tree = placard_layout::build(&dom, &styles, &font, viewport_width);

    println!("viewport_width = {viewport_width}");
    dump(&tree, tree.root(), 0);
}
