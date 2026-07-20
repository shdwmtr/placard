use crate::Fetcher;
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

fn validate_data_url(url: &str) -> Result<&str, String> {
    if url.is_empty() {
        return Err("'url' parameter must not be empty".to_string());
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("'url' parameter must be a well-formed http:// or https:// URL".to_string());
    }
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(
            "'url' parameter contains disallowed whitespace or control characters".to_string(),
        );
    }
    Ok(url)
}

// A deliberately small XML parser (elements, attributes, text, CDATA,
// comments, the XML declaration/DOCTYPE, and the five standard entities)
// paired with a small subset of XPath: absolute paths (`/root/child`),
// a single leading `//` for "search anywhere", `[n]` 1-based positional
// predicates, a trailing `@attr` for an attribute value, and a trailing
// `text()` for element text (the default when no trailing selector is
// given). Namespaces, other predicates (`[@id='x']`), XPath functions,
// and `//` occurring anywhere but the start of the path are not supported.
#[derive(Debug, Clone)]
enum XmlNode {
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<XmlNode>,
    },
    Text(String),
}

fn decode_entities(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        let mut entity = String::new();
        let mut closed = false;
        while let Some(&nc) = chars.peek() {
            if nc == ';' {
                chars.next();
                closed = true;
                break;
            }
            entity.push(nc);
            chars.next();
            if entity.len() > 10 {
                break;
            }
        }
        if !closed {
            out.push('&');
            out.push_str(&entity);
            continue;
        }
        match entity.as_str() {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                if let Ok(code) = u32::from_str_radix(&entity[2..], 16) {
                    out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                }
            }
            _ if entity.starts_with('#') => {
                if let Ok(code) = entity[1..].parse::<u32>() {
                    out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                }
            }
            _ => {
                out.push('&');
                out.push_str(&entity);
                out.push(';');
            }
        }
    }
    out
}

struct XmlParser<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> XmlParser<'a> {
    fn skip_ws(&mut self) {
        while matches!(self.chars.peek(), Some(c) if c.is_whitespace()) {
            self.chars.next();
        }
    }

    fn peek_str(&self, n: usize) -> String {
        self.chars.clone().take(n).collect()
    }

    fn skip_until(&mut self, terminator: &str) -> Result<(), String> {
        let term: Vec<char> = terminator.chars().collect();
        loop {
            if self.peek_str(term.len()).chars().collect::<Vec<_>>() == term {
                for _ in 0..term.len() {
                    self.chars.next();
                }
                return Ok(());
            }
            if self.chars.next().is_none() {
                return Err(format!("unterminated construct, expected '{terminator}'"));
            }
        }
    }

    fn skip_prolog_and_misc(&mut self) -> Result<(), String> {
        loop {
            self.skip_ws();
            if self.peek_str(2) == "<?" {
                self.skip_until("?>")?;
            } else if self.peek_str(4) == "<!--" {
                self.skip_until("-->")?;
            } else if self.peek_str(2) == "<!" {
                self.skip_until(">")?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn parse_name(&mut self) -> Result<String, String> {
        let mut name = String::new();
        while matches!(self.chars.peek(), Some(c) if c.is_alphanumeric() || matches!(c, '_' | '-' | ':' | '.'))
        {
            name.push(self.chars.next().unwrap());
        }
        if name.is_empty() {
            return Err("expected an element or attribute name".to_string());
        }
        Ok(name)
    }

    fn parse_attr_value(&mut self) -> Result<String, String> {
        let quote = self.chars.next().ok_or("expected quoted attribute value")?;
        if quote != '"' && quote != '\'' {
            return Err("expected a quoted attribute value".to_string());
        }
        let mut raw = String::new();
        loop {
            match self.chars.next() {
                Some(c) if c == quote => break,
                Some(c) => raw.push(c),
                None => return Err("unterminated attribute value".to_string()),
            }
        }
        Ok(decode_entities(&raw))
    }

    fn parse_text(&mut self) -> Result<String, String> {
        let mut raw = String::new();
        while matches!(self.chars.peek(), Some(c) if *c != '<') {
            raw.push(self.chars.next().unwrap());
        }
        Ok(decode_entities(&raw))
    }

    fn parse_element(&mut self) -> Result<XmlNode, String> {
        self.skip_ws();
        if self.chars.next() != Some('<') {
            return Err("expected '<' to start an element".to_string());
        }
        let tag = self.parse_name()?;
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.chars.peek() {
                Some('/') => {
                    self.chars.next();
                    if self.chars.next() != Some('>') {
                        return Err("expected '/>' to close a self-closing tag".to_string());
                    }
                    return Ok(XmlNode::Element {
                        tag,
                        attrs,
                        children: Vec::new(),
                    });
                }
                Some('>') => {
                    self.chars.next();
                    break;
                }
                Some(_) => {
                    let name = self.parse_name()?;
                    self.skip_ws();
                    if self.chars.next() != Some('=') {
                        return Err(format!("expected '=' after attribute name '{name}'"));
                    }
                    self.skip_ws();
                    let value = self.parse_attr_value()?;
                    attrs.push((name, value));
                }
                None => return Err("unexpected end of input in start tag".to_string()),
            }
        }

        let mut children = Vec::new();
        loop {
            if self.peek_str(4) == "<!--" {
                self.skip_until("-->")?;
                continue;
            }
            match self.chars.peek() {
                Some('<') => {
                    if self.peek_str(2) == "</" {
                        for _ in 0..2 {
                            self.chars.next();
                        }
                        let close_tag = self.parse_name()?;
                        self.skip_ws();
                        if self.chars.next() != Some('>') {
                            return Err(format!("expected '>' to close tag '{close_tag}'"));
                        }
                        if close_tag != tag {
                            return Err(format!(
                                "mismatched closing tag: expected '{tag}', found '{close_tag}'"
                            ));
                        }
                        break;
                    } else if self.peek_str(9) == "<![CDATA[" {
                        for _ in 0..9 {
                            self.chars.next();
                        }
                        let mut text = String::new();
                        loop {
                            if self.peek_str(3) == "]]>" {
                                for _ in 0..3 {
                                    self.chars.next();
                                }
                                break;
                            }
                            match self.chars.next() {
                                Some(c) => text.push(c),
                                None => return Err("unterminated CDATA section".to_string()),
                            }
                        }
                        children.push(XmlNode::Text(text));
                    } else {
                        children.push(self.parse_element()?);
                    }
                }
                Some(_) => {
                    let text = self.parse_text()?;
                    if !text.trim().is_empty() {
                        children.push(XmlNode::Text(text));
                    }
                }
                None => return Err(format!("unexpected end of input inside element '{tag}'")),
            }
        }
        Ok(XmlNode::Element {
            tag,
            attrs,
            children,
        })
    }
}

fn parse_xml(input: &str) -> Result<XmlNode, String> {
    let mut p = XmlParser {
        chars: input.chars().peekable(),
    };
    p.skip_prolog_and_misc()?;
    p.parse_element()
}

enum XPathStep {
    Tag { name: String, index: Option<usize> },
}

enum XPathTail {
    Attr(String),
    Text,
    None,
}

struct ParsedPath {
    anywhere_first: bool,
    steps: Vec<XPathStep>,
    tail: XPathTail,
}

fn parse_xpath(query: &str) -> Result<ParsedPath, String> {
    let mut q = query.trim();
    let anywhere_first = q.starts_with("//");
    if anywhere_first {
        q = &q[2..];
    } else if let Some(stripped) = q.strip_prefix('/') {
        q = stripped;
    } else {
        return Err(
            "query must be an absolute XPath expression starting with '/' or '//'".to_string(),
        );
    }

    if q.contains("//") {
        return Err(
            "only a leading '//' is supported; '//' elsewhere in the path is not supported"
                .to_string(),
        );
    }

    let mut parts: Vec<&str> = q.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err("query must select at least one element".to_string());
    }

    let mut tail = XPathTail::None;
    if let Some(last) = parts.last() {
        if *last == "text()" {
            tail = XPathTail::Text;
            parts.pop();
        } else if let Some(attr) = last.strip_prefix('@') {
            tail = XPathTail::Attr(attr.to_string());
            parts.pop();
        }
    }

    let mut steps = Vec::new();
    for part in parts {
        if let Some(bracket) = part.find('[') {
            if !part.ends_with(']') {
                return Err(format!("unsupported XPath step '{part}'"));
            }
            let name = &part[..bracket];
            let idx_str = &part[bracket + 1..part.len() - 1];
            let index: usize = idx_str.parse().map_err(|_| {
                format!(
                    "unsupported XPath predicate '[{idx_str}]'; only numeric indices are supported"
                )
            })?;
            if index == 0 {
                return Err("XPath indices are 1-based; '[0]' is not valid".to_string());
            }
            steps.push(XPathStep::Tag {
                name: name.to_string(),
                index: Some(index),
            });
        } else {
            steps.push(XPathStep::Tag {
                name: part.to_string(),
                index: None,
            });
        }
    }

    if steps.is_empty() && matches!(tail, XPathTail::None) {
        return Err("query must select at least one element".to_string());
    }

    Ok(ParsedPath {
        anywhere_first,
        steps,
        tail,
    })
}

fn find_descendants_by_tag<'a>(node: &'a XmlNode, tag: &str, out: &mut Vec<&'a XmlNode>) {
    if let XmlNode::Element {
        tag: t, children, ..
    } = node
    {
        if t == tag {
            out.push(node);
        }
        for child in children {
            find_descendants_by_tag(child, tag, out);
        }
    }
}

fn nth_child_by_tag<'a>(node: &'a XmlNode, tag: &str, index: usize) -> Option<&'a XmlNode> {
    if let XmlNode::Element { children, .. } = node {
        children
            .iter()
            .filter(|c| matches!(c, XmlNode::Element { tag: t, .. } if t == tag))
            .nth(index - 1)
    } else {
        None
    }
}

fn element_text(node: &XmlNode) -> String {
    if let XmlNode::Element { children, .. } = node {
        children
            .iter()
            .filter_map(|c| match c {
                XmlNode::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        String::new()
    }
}

fn evaluate(root: &XmlNode, path: &ParsedPath) -> Result<String, String> {
    let mut steps_iter = path.steps.iter();
    let mut context: &XmlNode;

    if path.anywhere_first {
        let XPathStep::Tag { name, index } = steps_iter
            .next()
            .ok_or("query must select at least one element")?;
        let mut matches = Vec::new();
        find_descendants_by_tag(root, name, &mut matches);
        let idx = index.unwrap_or(1);
        context = *matches.get(idx - 1).ok_or("no result")?;
    } else if let Some(XPathStep::Tag { name, index }) = steps_iter.next() {
        if let XmlNode::Element { tag, .. } = root {
            if tag != name || index.unwrap_or(1) != 1 {
                return Err("no result".to_string());
            }
            context = root;
        } else {
            return Err("no result".to_string());
        }
    } else {
        context = root;
    }

    for XPathStep::Tag { name, index } in steps_iter {
        let idx = index.unwrap_or(1);
        context = nth_child_by_tag(context, name, idx).ok_or("no result")?;
    }

    match &path.tail {
        XPathTail::Attr(attr) => {
            if let XmlNode::Element { attrs, .. } = context {
                attrs
                    .iter()
                    .find(|(k, _)| k == attr)
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| "no result".to_string())
            } else {
                Err("no result".to_string())
            }
        }
        XPathTail::Text | XPathTail::None => {
            let text = element_text(context).trim().to_string();
            if text.is_empty() {
                Err("no result".to_string())
            } else {
                Ok(text)
            }
        }
    }
}

pub(crate) fn resolve_xml(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("dynamic-xml requires a data-url attribute")?;
    let url = validate_data_url(url)?;
    let query = params
        .get("query")
        .ok_or("dynamic-xml requires a data-query attribute")?;
    if query.is_empty() {
        return Err("'query' parameter must not be empty".to_string());
    }
    let path = parse_xpath(query)?;

    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "dynamic-xml response was not valid UTF-8".to_string())?;
    let root = parse_xml(&text)?;
    let value = evaluate(&root, &path)?;

    let prefix = params.get("prefix").map(String::as_str).unwrap_or("");
    let suffix = params.get("suffix").map(String::as_str).unwrap_or("");
    Ok(format!("{prefix}{value}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://example.com/data.xml");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(query: &str) -> HashMap<String, String> {
        HashMap::from([
            (
                "url".to_string(),
                "https://example.com/data.xml".to_string(),
            ),
            ("query".to_string(), query.to_string()),
        ])
    }

    #[test]
    fn extracts_text_from_a_leading_double_slash_path() {
        let fetcher = FakeFetcher(
            r#"<slideshow><slide><title>Wake up to WonderWidgets!</title></slide></slideshow>"#,
        );
        let value = resolve_xml(&params("//slideshow/slide[1]/title"), &fetcher).unwrap();
        assert_eq!(value, "Wake up to WonderWidgets!");
    }

    #[test]
    fn extracts_an_attribute_value() {
        let fetcher = FakeFetcher(r#"<root><user id="42">alice</user></root>"#);
        let value = resolve_xml(&params("/root/user/@id"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_url_and_query_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_xml(&HashMap::new(), &Unused).is_err());
        assert!(resolve_xml(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        let mut p = params("/root/title");
        p.insert("url".to_string(), "file:///etc/passwd".to_string());
        assert!(resolve_xml(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_query_does_not_match() {
        let fetcher = FakeFetcher(r#"<root><title>hi</title></root>"#);
        assert!(resolve_xml(&params("/root/missing"), &fetcher).is_err());
    }

    #[test]
    fn rejects_relative_queries() {
        let fetcher = FakeFetcher(r#"<root><title>hi</title></root>"#);
        assert!(resolve_xml(&params("root/title"), &fetcher).is_err());
    }
}
