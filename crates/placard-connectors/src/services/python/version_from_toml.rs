use crate::Fetcher;
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

// A deliberately small ad hoc reader: we only need to find the `[project]`
// table and its `requires-python` key, not a general TOML document.
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

fn parse_toml_string(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        let inner = &raw[1..raw.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        Ok(out)
    } else if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        Ok(raw[1..raw.len() - 1].to_string())
    } else {
        Err(format!(
            "expected a quoted TOML string value, found '{raw}'"
        ))
    }
}

fn extract_requires_python(toml: &str) -> Result<String, String> {
    let mut in_project_table = false;
    for raw_line in toml.lines() {
        let line = strip_toml_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_project_table = line == "[project]";
            continue;
        }
        if !in_project_table {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim().trim_matches('"').trim_matches('\'');
        if key == "requires-python" {
            return parse_toml_string(line[eq + 1..].trim());
        }
    }
    Err("pyproject.toml is missing project.requires-python".to_string())
}

pub(crate) fn resolve_version_from_toml(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("python-version-from-toml requires a data-url attribute")?;
    let url = validate_data_url(url)?;

    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "python-version-from-toml response was not valid UTF-8".to_string())?;
    extract_requires_python(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/numpy/numpy/main/pyproject.toml"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([(
            "url".to_string(),
            "https://raw.githubusercontent.com/numpy/numpy/main/pyproject.toml".to_string(),
        )])
    }

    #[test]
    fn extracts_requires_python_from_the_project_table() {
        let fetcher = FakeFetcher(
            "[build-system]\nrequires = [\"setuptools\"]\n\n[project]\nname = \"numpy\"\nrequires-python = \">=3.10\"\n",
        );
        let value = resolve_version_from_toml(&params(), &fetcher).unwrap();
        assert_eq!(value, ">=3.10");
    }

    #[test]
    fn stops_matching_once_a_new_table_starts() {
        let fetcher = FakeFetcher(
            "[project]\nname = \"numpy\"\n\n[tool.other]\nrequires-python = \">=2.7\"\n",
        );
        assert!(resolve_version_from_toml(&params(), &fetcher).is_err());
    }

    #[test]
    fn requires_the_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version_from_toml(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_version_from_toml(
                &HashMap::from([("url".to_string(), String::new())]),
                &Unused
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        let p = HashMap::from([("url".to_string(), "file:///etc/passwd".to_string())]);
        assert!(resolve_version_from_toml(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher("[project]\nname = \"numpy\"\n");
        assert!(resolve_version_from_toml(&params(), &fetcher).is_err());
    }
}
