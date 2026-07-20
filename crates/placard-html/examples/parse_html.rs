use placard_html::{Dom, NodeData, NodeId};

fn dump(dom: &Dom, id: NodeId, depth: usize) {
    let indent = "  ".repeat(depth);
    match dom.data(id) {
        NodeData::Document => {
            for child in dom.children(id) {
                dump(dom, child, depth);
            }
            return;
        }
        NodeData::Element { tag, attrs } => {
            let attrs_str: String = attrs.iter().map(|(k, v)| format!(" {k}={v:?}")).collect();
            println!("{indent}<{tag}{attrs_str}>");
        }
        NodeData::Text(text) => {
            println!("{indent}{text:?}");
        }
    }
    for child in dom.children(id) {
        dump(dom, child, depth + 1);
    }
}

fn run(label: &str, html: &str) {
    println!("=== {label} ===");
    println!("input: {html}");
    let dom = placard_html::parse(html);
    dump(&dom, dom.root(), 0);
    println!();
}

fn main() {
    run(
        "nested divs",
        r#"<div class="outer"><div class="inner">Hello <b>world</b></div></div>"#,
    );

    run(
        "unclosed p followed by another p",
        "<p>First paragraph<p>Second paragraph",
    );

    run(
        "self-closing and void img",
        r#"<div><img src="a.png"/><img src="b.png">after</div>"#,
    );

    run("entity in text", "<p>Fish &amp; Chips &lt;tasty&gt;</p>");

    run(
        "li implicit close inside ul",
        "<ul><li>one<li>two<li>three</ul>",
    );

    run(
        "inline style raw text not tokenized as tags",
        "<style>div > p { color: red; } /* <fake-tag> */</style><div>after</div>",
    );

    run(
        "comment is discarded",
        "<p>before<!-- a comment --> after</p>",
    );
}
