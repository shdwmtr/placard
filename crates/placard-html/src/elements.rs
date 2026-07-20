pub fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

pub fn is_raw_text_element(tag: &str) -> bool {
    matches!(tag, "script" | "style")
}

pub fn implicitly_closes(open_tag: &str, new_tag: &str) -> bool {
    match open_tag {
        "p" => matches!(
            new_tag,
            "p" | "div" | "ul" | "ol" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        ),
        "li" => new_tag == "li",
        _ => false,
    }
}
