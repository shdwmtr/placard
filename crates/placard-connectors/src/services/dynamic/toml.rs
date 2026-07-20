use crate::Fetcher;
use crate::json::Value;
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

// A deliberately small subset of TOML: `[table]` / `[a.b]` headers, dotted
// `key = value` pairs, basic/literal single-line strings, integers, floats,
// booleans, single-line arrays, and single-line inline tables. Array-of-tables
// (`[[table]]`) headers, multi-line strings/arrays, and native date-time
// values are not supported (the last is accepted only as a plain string).
fn strip_toml_comment(line: &str) -> &str {
    let mut in_quote: Option<char> = None;
    for (idx, c) in line.char_indices() {
        match in_quote {
            Some(q) => {
                if c == q {
                    in_quote = None;
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    in_quote = Some(c);
                } else if c == '#' {
                    return &line[..idx];
                }
            }
        }
    }
    line
}

fn split_dotted_key(raw: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;
    for c in raw.chars() {
        match in_quote {
            Some(q) => {
                if c == q {
                    in_quote = None;
                } else {
                    current.push(c);
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    in_quote = Some(c);
                } else if c == '.' {
                    parts.push(current.trim().to_string());
                    current = String::new();
                } else {
                    current.push(c);
                }
            }
        }
    }
    parts.push(current.trim().to_string());
    if parts.iter().any(|p| p.is_empty()) {
        return Err(format!("invalid TOML key '{raw}'"));
    }
    Ok(parts)
}

fn ensure_table_path(root: &mut Vec<(String, Value)>, path: &[String]) -> Result<(), String> {
    if path.is_empty() {
        return Ok(());
    }
    let key = &path[0];
    if let Some(idx) = root.iter().position(|(k, _)| k == key) {
        match &mut root[idx].1 {
            Value::Object(fields) => ensure_table_path(fields, &path[1..]),
            _ => Err(format!("key '{key}' redefined as a table")),
        }
    } else {
        root.push((key.clone(), Value::Object(Vec::new())));
        let idx = root.len() - 1;
        if let Value::Object(fields) = &mut root[idx].1 {
            ensure_table_path(fields, &path[1..])
        } else {
            unreachable!()
        }
    }
}

fn set_path(root: &mut Vec<(String, Value)>, path: &[String], value: Value) -> Result<(), String> {
    if path.len() == 1 {
        let key = &path[0];
        if root.iter().any(|(k, _)| k == key) {
            return Err(format!("key '{key}' is defined more than once"));
        }
        root.push((key.clone(), value));
        return Ok(());
    }
    let key = &path[0];
    if let Some(idx) = root.iter().position(|(k, _)| k == key) {
        match &mut root[idx].1 {
            Value::Object(fields) => set_path(fields, &path[1..], value),
            _ => Err(format!("key '{key}' is not a table")),
        }
    } else {
        root.push((key.clone(), Value::Object(Vec::new())));
        let idx = root.len() - 1;
        if let Value::Object(fields) = &mut root[idx].1 {
            set_path(fields, &path[1..], value)
        } else {
            unreachable!()
        }
    }
}

struct TomlValueParser<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> TomlValueParser<'a> {
    fn skip_ws(&mut self) {
        while matches!(self.chars.peek(), Some(' ' | '\t')) {
            self.chars.next();
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.chars.peek() {
            Some('"') => self.parse_basic_string().map(Value::String),
            Some('\'') => self.parse_literal_string().map(Value::String),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_inline_table(),
            Some('t') | Some('f') => self.parse_bool(),
            Some(c) if c.is_ascii_digit() || *c == '-' || *c == '+' => self.parse_number(),
            Some(c) => Err(format!("unsupported TOML value starting with '{c}'")),
            None => Err("unexpected end of TOML value".to_string()),
        }
    }

    fn parse_basic_string(&mut self) -> Result<String, String> {
        self.chars.next();
        let mut out = String::new();
        loop {
            match self.chars.next() {
                Some('"') => break,
                Some('\\') => match self.chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some('u') => {
                        let mut code = 0u32;
                        for _ in 0..4 {
                            let c = self.chars.next().ok_or("unexpected end of string escape")?;
                            let d = c
                                .to_digit(16)
                                .ok_or_else(|| format!("invalid hex digit '{c}'"))?;
                            code = code * 16 + d;
                        }
                        out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                    }
                    Some(c) => return Err(format!("invalid escape '\\{c}' in TOML string")),
                    None => return Err("unterminated escape in TOML string".to_string()),
                },
                Some(c) => out.push(c),
                None => return Err("unterminated TOML string".to_string()),
            }
        }
        Ok(out)
    }

    fn parse_literal_string(&mut self) -> Result<String, String> {
        self.chars.next();
        let mut out = String::new();
        loop {
            match self.chars.next() {
                Some('\'') => break,
                Some(c) => out.push(c),
                None => return Err("unterminated TOML literal string".to_string()),
            }
        }
        Ok(out)
    }

    fn parse_bool(&mut self) -> Result<Value, String> {
        let rest: String = self.chars.clone().collect();
        if rest.starts_with("true") {
            for _ in 0.."true".len() {
                self.chars.next();
            }
            Ok(Value::Bool(true))
        } else if rest.starts_with("false") {
            for _ in 0.."false".len() {
                self.chars.next();
            }
            Ok(Value::Bool(false))
        } else {
            Err("expected 'true' or 'false' in TOML value".to_string())
        }
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let mut text = String::new();
        while matches!(self.chars.peek(), Some(c) if c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | '_' | 'e' | 'E'))
        {
            let c = self.chars.next().unwrap();
            if c != '_' {
                text.push(c);
            }
        }
        if matches!(self.chars.peek(), Some(c) if c.is_ascii_alphabetic() || *c == ':') {
            while matches!(self.chars.peek(), Some(c) if !matches!(c, ',' | ']' | '}' | ' ' | '\t'))
            {
                text.push(self.chars.next().unwrap());
            }
            return Ok(Value::String(text));
        }
        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| format!("invalid TOML number '{text}'"))
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.chars.next();
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.chars.peek() == Some(&']') {
                self.chars.next();
                break;
            }
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.chars.peek() {
                Some(',') => {
                    self.chars.next();
                }
                Some(']') => {
                    self.chars.next();
                    break;
                }
                _ => return Err("expected ',' or ']' in TOML array".to_string()),
            }
        }
        Ok(Value::Array(items))
    }

    fn parse_inline_table(&mut self) -> Result<Value, String> {
        self.chars.next();
        let mut fields = Vec::new();
        self.skip_ws();
        if self.chars.peek() == Some(&'}') {
            self.chars.next();
            return Ok(Value::Object(fields));
        }
        loop {
            self.skip_ws();
            let mut key = String::new();
            while matches!(self.chars.peek(), Some(c) if *c != '=' && !c.is_whitespace()) {
                key.push(self.chars.next().unwrap());
            }
            self.skip_ws();
            if self.chars.next() != Some('=') {
                return Err("expected '=' in TOML inline table".to_string());
            }
            self.skip_ws();
            let value = self.parse_value()?;
            fields.push((key, value));
            self.skip_ws();
            match self.chars.peek() {
                Some(',') => {
                    self.chars.next();
                }
                Some('}') => {
                    self.chars.next();
                    break;
                }
                _ => return Err("expected ',' or '}' in TOML inline table".to_string()),
            }
        }
        Ok(Value::Object(fields))
    }
}

fn parse_toml_value(raw: &str) -> Result<Value, String> {
    let mut parser = TomlValueParser {
        chars: raw.trim().chars().peekable(),
    };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.chars.peek().is_some() {
        return Err(format!("unexpected trailing data in TOML value '{raw}'"));
    }
    Ok(value)
}

fn parse_toml(input: &str) -> Result<Value, String> {
    let mut root: Vec<(String, Value)> = Vec::new();
    let mut current_path: Vec<String> = Vec::new();

    for raw_line in input.lines() {
        let line = strip_toml_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("[[") {
            return Err("array-of-tables ([[table]]) headers are not supported".to_string());
        } else if line.starts_with('[') && line.ends_with(']') {
            let inner = &line[1..line.len() - 1];
            current_path = split_dotted_key(inner)?;
            ensure_table_path(&mut root, &current_path)?;
        } else {
            let eq = line
                .find('=')
                .ok_or_else(|| format!("expected 'key = value' in TOML line '{line}'"))?;
            let key_part = line[..eq].trim();
            let value_part = line[eq + 1..].trim();
            let key_path = split_dotted_key(key_part)?;
            let value = parse_toml_value(value_part)?;
            let mut full_path = current_path.clone();
            full_path.extend(key_path);
            set_path(&mut root, &full_path, value)?;
        }
    }

    Ok(Value::Object(root))
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

pub(crate) fn resolve_toml(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("dynamic-toml requires a data-url attribute")?;
    let url = validate_data_url(url)?;
    let query = params
        .get("query")
        .ok_or("dynamic-toml requires a data-query attribute")?;
    if query.is_empty() {
        return Err("'query' parameter must not be empty".to_string());
    }
    let segments = parse_query(query)?;

    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "dynamic-toml response was not valid UTF-8".to_string())?;
    let root = parse_toml(&text)?;
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
            assert_eq!(url, "https://example.com/Cargo.toml");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(query: &str) -> HashMap<String, String> {
        HashMap::from([
            (
                "url".to_string(),
                "https://example.com/Cargo.toml".to_string(),
            ),
            ("query".to_string(), query.to_string()),
        ])
    }

    #[test]
    fn extracts_a_top_level_key() {
        let fetcher = FakeFetcher("title = \"TOML Example\"\n");
        let value = resolve_toml(&params("$.title"), &fetcher).unwrap();
        assert_eq!(value, "TOML Example");
    }

    #[test]
    fn extracts_a_nested_table_key_and_array_item() {
        let fetcher = FakeFetcher(
            "[package]\nname = \"placard\"\nversion = \"1.2.3\"\n\n[package.metadata]\ntags = [\"a\", \"b\"]\n",
        );
        assert_eq!(
            resolve_toml(&params("$.package.name"), &fetcher).unwrap(),
            "placard"
        );
        assert_eq!(
            resolve_toml(&params("$.package.metadata.tags[1]"), &fetcher).unwrap(),
            "b"
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
        assert!(resolve_toml(&HashMap::new(), &Unused).is_err());
        assert!(resolve_toml(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        let mut p = params("$.title");
        p.insert("url".to_string(), "file:///etc/passwd".to_string());
        assert!(resolve_toml(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_query_does_not_match() {
        let fetcher = FakeFetcher("title = \"TOML Example\"\n");
        assert!(resolve_toml(&params("$.missing"), &fetcher).is_err());
    }

    #[test]
    fn rejects_array_of_tables() {
        let fetcher = FakeFetcher("[[products]]\nname = \"x\"\n");
        assert!(resolve_toml(&params("$.products"), &fetcher).is_err());
    }
}
