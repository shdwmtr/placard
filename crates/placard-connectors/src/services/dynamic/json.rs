use crate::Fetcher;
use crate::json;
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

fn eval_query<'a>(value: &'a json::Value, segments: &[PathSegment]) -> Option<&'a json::Value> {
    let mut current = value;
    for segment in segments {
        current = match (segment, current) {
            (PathSegment::Key(key), json::Value::Object(fields)) => {
                &fields.iter().find(|(k, _)| k == key)?.1
            }
            (PathSegment::Index(idx), json::Value::Array(items)) => items.get(*idx)?,
            _ => return None,
        };
    }
    Some(current)
}

pub(crate) fn resolve_json(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("dynamic-json requires a data-url attribute")?;
    let url = validate_data_url(url)?;
    let query = params
        .get("query")
        .ok_or("dynamic-json requires a data-query attribute")?;
    if query.is_empty() {
        return Err("'query' parameter must not be empty".to_string());
    }
    let segments = parse_query(query)?;

    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "dynamic-json response was not valid UTF-8".to_string())?;
    let root = json::parse(&text)?;
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
            assert_eq!(url, "https://example.com/data.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(url: &str, query: &str) -> HashMap<String, String> {
        HashMap::from([
            ("url".to_string(), url.to_string()),
            ("query".to_string(), query.to_string()),
        ])
    }

    #[test]
    fn extracts_a_dotted_field() {
        let fetcher = FakeFetcher(r#"{"name": "placard", "meta": {"count": 42}}"#);
        let value = resolve_json(
            &params("https://example.com/data.json", "$.meta.count"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn extracts_an_array_index() {
        let fetcher = FakeFetcher(r#"{"items": [{"name": "first"}, {"name": "second"}]}"#);
        let value = resolve_json(
            &params("https://example.com/data.json", "$.items[1].name"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "second");
    }

    #[test]
    fn applies_prefix_and_suffix() {
        let fetcher = FakeFetcher(r#"{"version": "1.2.3"}"#);
        let mut p = params("https://example.com/data.json", "$.version");
        p.insert("prefix".to_string(), "v".to_string());
        p.insert("suffix".to_string(), "!".to_string());
        let value = resolve_json(&p, &fetcher).unwrap();
        assert_eq!(value, "v1.2.3!");
    }

    #[test]
    fn requires_url_and_query_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_json(&HashMap::new(), &Unused).is_err());
        assert!(resolve_json(&params("https://example.com/data.json", ""), &Unused).is_err());
        assert!(resolve_json(&params("", "$.a"), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        assert!(resolve_json(&params("file:///etc/passwd", "$.a"), &Unused).is_err());
        assert!(resolve_json(&params("javascript:alert(1)", "$.a"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_query_does_not_match() {
        let fetcher = FakeFetcher(r#"{"name": "placard"}"#);
        assert!(
            resolve_json(
                &params("https://example.com/data.json", "$.missing"),
                &fetcher
            )
            .is_err()
        );
    }
}
