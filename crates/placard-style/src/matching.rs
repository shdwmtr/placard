use placard_css::{AttrMatch, Combinator, Selector, SimpleSelector};
use placard_html::{Dom, NodeId};
use std::collections::HashSet;

fn simple_selector_matches(dom: &Dom, node: NodeId, sel: &SimpleSelector) -> bool {
    let Some(tag) = dom.tag(node) else {
        return false;
    };

    if let Some(want_tag) = &sel.tag {
        if want_tag != tag {
            return false;
        }
    }

    if let Some(want_id) = &sel.id {
        if dom.attr(node, "id") != Some(want_id.as_str()) {
            return false;
        }
    }

    if !sel.classes.is_empty() {
        let class_attr = dom.attr(node, "class").unwrap_or("");
        let node_classes: HashSet<&str> = class_attr.split_whitespace().collect();
        if !sel
            .classes
            .iter()
            .all(|want| node_classes.contains(want.as_str()))
        {
            return false;
        }
    }

    for (name, matcher) in &sel.attrs {
        match matcher {
            AttrMatch::Present => {
                if dom.attr(node, name).is_none() {
                    return false;
                }
            }
            AttrMatch::Equals(value) => {
                if dom.attr(node, name) != Some(value.as_str()) {
                    return false;
                }
            }
        }
    }

    true
}

fn prev_element_sibling(dom: &Dom, node: NodeId) -> Option<NodeId> {
    let mut prev = dom.prev_sibling(node);
    while let Some(candidate) = prev {
        if dom.tag(candidate).is_some() {
            return Some(candidate);
        }
        prev = dom.prev_sibling(candidate);
    }
    None
}

pub fn selector_matches(dom: &Dom, node: NodeId, selector: &Selector) -> bool {
    if selector.parts.is_empty() {
        return false;
    }

    let mut part_idx = selector.parts.len() - 1;
    if !simple_selector_matches(dom, node, &selector.parts[part_idx]) {
        return false;
    }

    let mut current = node;
    while part_idx > 0 {
        let combinator = selector.combinators[part_idx - 1];
        let target = &selector.parts[part_idx - 1];

        match combinator {
            Combinator::Child => {
                let Some(parent) = dom.parent(current) else {
                    return false;
                };
                if !simple_selector_matches(dom, parent, target) {
                    return false;
                }
                current = parent;
            }
            Combinator::Descendant => {
                let mut ancestor = dom.parent(current);
                let mut found = None;
                while let Some(a) = ancestor {
                    if simple_selector_matches(dom, a, target) {
                        found = Some(a);
                        break;
                    }
                    ancestor = dom.parent(a);
                }
                let Some(matched) = found else {
                    return false;
                };
                current = matched;
            }
            Combinator::Adjacent => {
                let Some(prev) = prev_element_sibling(dom, current) else {
                    return false;
                };
                if !simple_selector_matches(dom, prev, target) {
                    return false;
                }
                current = prev;
            }
            Combinator::General => {
                let mut sibling = prev_element_sibling(dom, current);
                let mut found = None;
                while let Some(s) = sibling {
                    if simple_selector_matches(dom, s, target) {
                        found = Some(s);
                        break;
                    }
                    sibling = prev_element_sibling(dom, s);
                }
                let Some(matched) = found else {
                    return false;
                };
                current = matched;
            }
        }
        part_idx -= 1;
    }

    true
}
