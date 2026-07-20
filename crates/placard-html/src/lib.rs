mod dom;
mod elements;
mod tokenizer;

pub use dom::{Children, Dom, NodeData, NodeId};

use elements::{implicitly_closes, is_raw_text_element, is_void_element};
use tokenizer::{Token, Tokenizer};

const MAX_NESTING_DEPTH: usize = 200;

pub fn parse(input: &str) -> Dom {
    let mut dom = Dom::new();
    let root = dom.root();
    let mut stack: Vec<NodeId> = vec![root];
    let mut tokenizer = Tokenizer::new(input);

    while let Some(token) = tokenizer.next_token() {
        match token {
            Token::Text(text) => {
                if !text.is_empty() {
                    let parent = *stack.last().unwrap();
                    dom.append_text(parent, &text);
                }
            }
            Token::StartTag {
                name,
                attrs,
                self_closing,
            } => {
                while stack.len() > 1 {
                    let top = *stack.last().unwrap();
                    let top_tag = dom.tag(top).unwrap_or("");
                    if implicitly_closes(top_tag, &name) {
                        stack.pop();
                    } else {
                        break;
                    }
                }

                let parent = *stack.last().unwrap();
                let node = dom.append_element(parent, &name, attrs);
                let self_closing = self_closing || is_void_element(&name);

                if is_raw_text_element(&name) && !self_closing {
                    let raw = tokenizer.read_raw_text_until(&name);
                    if !raw.is_empty() {
                        dom.append_text(node, &raw);
                    }
                    tokenizer.next_token();
                } else if !self_closing && stack.len() < MAX_NESTING_DEPTH {
                    stack.push(node);
                }
            }
            Token::EndTag { name } => {
                if let Some(pos) = stack
                    .iter()
                    .rposition(|&id| dom.tag(id) == Some(name.as_str()))
                {
                    if pos > 0 {
                        stack.truncate(pos);
                    }
                }
            }
        }
    }

    dom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child_tags(dom: &Dom, id: NodeId) -> Vec<String> {
        dom.children(id)
            .filter_map(|c| dom.tag(c).map(String::from))
            .collect()
    }

    #[test]
    fn implicit_p_close_produces_siblings_not_nesting() {
        let dom = parse("<p>First<p>Second");
        let root_children = child_tags(&dom, dom.root());
        assert_eq!(root_children, vec!["p", "p"]);
    }

    #[test]
    fn implicit_li_close_produces_siblings_not_nesting() {
        let dom = parse("<ul><li>one<li>two<li>three</ul>");
        let ul = dom.first_child(dom.root()).unwrap();
        assert_eq!(child_tags(&dom, ul), vec!["li", "li", "li"]);
    }

    #[test]
    fn void_element_does_not_swallow_following_sibling() {
        let dom = parse("<div><img src=\"a.png\">after</div>");
        let div = dom.first_child(dom.root()).unwrap();
        let img = dom.first_child(div).unwrap();
        assert_eq!(dom.tag(img), Some("img"));
        assert!(dom.first_child(img).is_none());
        let text_sibling = dom.next_sibling(img).unwrap();
        assert_eq!(dom.text(text_sibling), Some("after"));
    }

    #[test]
    fn self_closing_void_img_also_does_not_swallow_sibling() {
        let dom = parse("<div><img src=\"a.png\"/>after</div>");
        let div = dom.first_child(dom.root()).unwrap();
        assert_eq!(child_tags(&dom, div), vec!["img"]);
    }

    #[test]
    fn raw_text_element_preserves_literal_angle_brackets() {
        let dom = parse("<style>a > b { color: red; }</style><div>after</div>");
        let style = dom.first_child(dom.root()).unwrap();
        assert_eq!(dom.tag(style), Some("style"));
        let text = dom.first_child(style).unwrap();
        assert_eq!(dom.text(text), Some("a > b { color: red; }"));
        let div = dom.next_sibling(style).unwrap();
        assert_eq!(dom.tag(div), Some("div"));
    }

    #[test]
    fn entities_decode_in_text() {
        let dom = parse("<p>Fish &amp; Chips &lt;tasty&gt;</p>");
        let p = dom.first_child(dom.root()).unwrap();
        let text = dom.first_child(p).unwrap();
        assert_eq!(dom.text(text), Some("Fish & Chips <tasty>"));
    }

    #[test]
    fn comment_is_discarded() {
        let dom = parse("<p>before<!-- gone --></p>");
        let p = dom.first_child(dom.root()).unwrap();
        let text = dom.first_child(p).unwrap();
        assert_eq!(dom.text(text), Some("before"));
        assert!(dom.next_sibling(text).is_none());
    }

    #[test]
    fn nesting_beyond_the_depth_cap_is_flattened_instead_of_growing_forever() {
        let depth = 10_000;
        let html = format!("{}x{}", "<div>".repeat(depth), "</div>".repeat(depth));
        let dom = parse(&html);

        let mut max_depth = 0;
        let mut stack = vec![(dom.root(), 0)];
        while let Some((node, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);
            for child in dom.children(node) {
                stack.push((child, depth + 1));
            }
        }

        assert!(
            max_depth <= MAX_NESTING_DEPTH,
            "expected nesting to be capped at {MAX_NESTING_DEPTH}, got {max_depth}"
        );
    }
}
