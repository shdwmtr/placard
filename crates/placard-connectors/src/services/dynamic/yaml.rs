use crate::Fetcher;
use crate::json::Value;
use std::collections::HashMap;

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

// A deliberately small subset of YAML: block mappings and block sequences
// nested purely by indentation, plain/quoted scalars, and whole-line `#`
// comments. Flow collections (`{...}`/`[...]`), anchors/aliases, multi-line
// block scalars (`|`, `>`), multi-document streams, and tab indentation are
// not supported.
struct Line<'a> {
    indent: usize,
    content: &'a str,
}

fn preprocess(input: &str) -> Vec<Line<'_>> {
    input
        .lines()
        .filter_map(|line| {
            let trimmed_start = line.trim_start_matches(' ');
            let indent = line.len() - trimmed_start.len();
            let content = trimmed_start.trim_end();
            if content.is_empty() || content.starts_with('#') || content == "---" {
                return None;
            }
            Some(Line { indent, content })
        })
        .collect()
}

fn unquote(raw: &str) -> String {
    if raw.len() >= 2
        && ((raw.starts_with('"') && raw.ends_with('"'))
            || (raw.starts_with('\'') && raw.ends_with('\'')))
    {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

fn parse_scalar(raw: &str) -> Value {
    let raw = raw.trim();
    if raw.len() >= 2
        && ((raw.starts_with('"') && raw.ends_with('"'))
            || (raw.starts_with('\'') && raw.ends_with('\'')))
    {
        return Value::String(unquote(raw));
    }
    match raw {
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        "null" | "Null" | "NULL" | "~" | "" => return Value::Null,
        _ => {}
    }
    if let Ok(n) = raw.parse::<f64>() {
        return Value::Number(n);
    }
    Value::String(raw.to_string())
}

fn find_key_colon(content: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    for (idx, ch) in content.char_indices() {
        match in_quote {
            Some(q) => {
                if ch == q {
                    in_quote = None;
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    in_quote = Some(ch);
                } else if ch == ':' {
                    let next = content[idx + ch.len_utf8()..].chars().next();
                    if next.is_none() || next == Some(' ') {
                        return Some(idx);
                    }
                }
            }
        }
    }
    None
}

fn parse_block(lines: &[Line], start: usize, indent: usize) -> Result<(Value, usize), String> {
    if start >= lines.len() {
        return Ok((Value::Null, start));
    }
    if lines[start].content == "-" || lines[start].content.starts_with("- ") {
        parse_sequence(lines, start, indent)
    } else {
        parse_mapping(lines, start, indent)
    }
}

fn parse_sequence(lines: &[Line], mut i: usize, indent: usize) -> Result<(Value, usize), String> {
    let mut items = Vec::new();
    while i < lines.len()
        && lines[i].indent == indent
        && (lines[i].content == "-" || lines[i].content.starts_with("- "))
    {
        let rest = lines[i].content.strip_prefix('-').unwrap_or("").trim();
        if rest.is_empty() {
            if i + 1 < lines.len() && lines[i + 1].indent > indent {
                let (value, next) = parse_block(lines, i + 1, lines[i + 1].indent)?;
                items.push(value);
                i = next;
            } else {
                items.push(Value::Null);
                i += 1;
            }
        } else if find_key_colon(rest).is_some() {
            let synthetic_indent = indent + 2;
            let mut sub_lines = vec![Line {
                indent: synthetic_indent,
                content: rest,
            }];
            let mut j = i + 1;
            while j < lines.len() && lines[j].indent > indent {
                sub_lines.push(Line {
                    indent: lines[j].indent,
                    content: lines[j].content,
                });
                j += 1;
            }
            let (value, _) = parse_mapping(&sub_lines, 0, synthetic_indent)?;
            items.push(value);
            i = j;
        } else {
            items.push(parse_scalar(rest));
            i += 1;
        }
    }
    Ok((Value::Array(items), i))
}

fn parse_mapping(lines: &[Line], mut i: usize, indent: usize) -> Result<(Value, usize), String> {
    let mut fields = Vec::new();
    while i < lines.len() && lines[i].indent == indent {
        let content = lines[i].content;
        let colon = find_key_colon(content)
            .ok_or_else(|| format!("expected 'key: value' in YAML line '{content}'"))?;
        let key = unquote(content[..colon].trim());
        let rest = content[colon + 1..].trim();
        if rest.is_empty() {
            if i + 1 < lines.len() && lines[i + 1].indent > indent {
                let (value, next) = parse_block(lines, i + 1, lines[i + 1].indent)?;
                fields.push((key, value));
                i = next;
            } else {
                fields.push((key, Value::Null));
                i += 1;
            }
        } else {
            fields.push((key, parse_scalar(rest)));
            i += 1;
        }
    }
    Ok((Value::Object(fields), i))
}

fn parse_yaml(input: &str) -> Result<Value, String> {
    let lines = preprocess(input);
    if lines.is_empty() {
        return Err("empty YAML document".to_string());
    }
    let indent = lines[0].indent;
    let (value, consumed) = parse_block(&lines, 0, indent)?;
    if consumed != lines.len() {
        return Err(format!(
            "unexpected indentation at YAML line '{}'",
            lines[consumed].content
        ));
    }
    Ok(value)
}

#[derive(Debug, PartialEq)]
enum PathSegment {
    Key(String),
    Index(usize),
}

fn parse_query(query: &str) -> Result<Vec<PathSegment>, String> {
    let query = query.strip_prefix('$').unwrap_or(query);
    let chars: Vec<char> = query.chars().collect();
    let mut segments = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '.' => {
                if !buf.is_empty() {
                    segments.push(PathSegment::Key(std::mem::take(&mut buf)));
                }
                i += 1;
            }
            '[' => {
                if !buf.is_empty() {
                    segments.push(PathSegment::Key(std::mem::take(&mut buf)));
                }
                let close = chars[i..]
                    .iter()
                    .position(|&c| c == ']')
                    .map(|p| p + i)
                    .ok_or("unterminated '[' in query")?;
                let inner: String = chars[i + 1..close].iter().collect();
                let inner = inner.trim();
                if inner.len() >= 2
                    && ((inner.starts_with('\'') && inner.ends_with('\''))
                        || (inner.starts_with('"') && inner.ends_with('"')))
                {
                    segments.push(PathSegment::Key(inner[1..inner.len() - 1].to_string()));
                } else if let Ok(idx) = inner.parse::<usize>() {
                    segments.push(PathSegment::Index(idx));
                } else {
                    return Err(format!("unsupported query segment '[{inner}]'"));
                }
                i = close + 1;
            }
            c => {
                buf.push(c);
                i += 1;
            }
        }
    }
    if !buf.is_empty() {
        segments.push(PathSegment::Key(buf));
    }
    if segments.is_empty() {
        return Err("query must select at least one field".to_string());
    }
    Ok(segments)
}

fn eval_query<'a>(value: &'a Value, segments: &[PathSegment]) -> Option<&'a Value> {
    let mut current = value;
    for segment in segments {
        current = match (segment, current) {
            (PathSegment::Key(key), Value::Object(fields)) => {
                &fields.iter().find(|(k, _)| k == key)?.1
            }
            (PathSegment::Index(idx), Value::Array(items)) => items.get(*idx)?,
            _ => return None,
        };
    }
    Some(current)
}

pub(crate) fn resolve_yaml(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("dynamic-yaml requires a data-url attribute")?;
    let url = validate_data_url(url)?;
    let query = params
        .get("query")
        .ok_or("dynamic-yaml requires a data-query attribute")?;
    if query.is_empty() {
        return Err("'query' parameter must not be empty".to_string());
    }
    let segments = parse_query(query)?;

    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "dynamic-yaml response was not valid UTF-8".to_string())?;
    let root = parse_yaml(&text)?;
    let found =
        eval_query(&root, &segments).ok_or("query did not match any value in the response")?;
    let value = found
        .as_text()
        .ok_or_else(|| "matched value was not a plain scalar".to_string())?;

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
            assert_eq!(url, "https://example.com/values.yaml");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(query: &str) -> HashMap<String, String> {
        HashMap::from([
            (
                "url".to_string(),
                "https://example.com/values.yaml".to_string(),
            ),
            ("query".to_string(), query.to_string()),
        ])
    }

    #[test]
    fn extracts_a_top_level_flat_key() {
        let fetcher = FakeFetcher("version: 2\nname: placard\n");
        let value = resolve_yaml(&params("$.version"), &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn extracts_a_nested_key_and_a_sequence_item() {
        let fetcher =
            FakeFetcher("database:\n  host: localhost\n  ports:\n    - 8000\n    - 8001\n");
        assert_eq!(
            resolve_yaml(&params("$.database.host"), &fetcher).unwrap(),
            "localhost"
        );
        assert_eq!(
            resolve_yaml(&params("$.database.ports[1]"), &fetcher).unwrap(),
            "8001"
        );
    }

    #[test]
    fn requires_url_and_query_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_yaml(&HashMap::new(), &Unused).is_err());
        assert!(resolve_yaml(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        let mut p = params("$.version");
        p.insert("url".to_string(), "file:///etc/passwd".to_string());
        assert!(resolve_yaml(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_query_does_not_match() {
        let fetcher = FakeFetcher("version: 2\n");
        assert!(resolve_yaml(&params("$.missing"), &fetcher).is_err());
    }
}
