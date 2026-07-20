use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_project(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'project' parameter must not be empty".to_string());
    }
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err("'project' parameter contains disallowed characters".to_string());
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err("'project' parameter contains disallowed characters".to_string());
        }
    }
    Ok(value)
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

fn resolve_base_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("gitlab-url") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'gitlab-url' parameter must not be empty".to_string());
            }
            if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
                return Err("'gitlab-url' parameter must be an http(s) URL".to_string());
            }
            if trimmed
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
            {
                return Err("'gitlab-url' parameter contains disallowed characters".to_string());
            }
            Ok(trimmed.to_string())
        }
        None => Ok("https://gitlab.com".to_string()),
    }
}

/// Only `data-sort="date"` (the default) is supported -- picking the
/// "latest" tag by semver comparison across every tag on the project is
/// out of scope for a plain field-extraction preset.
pub(crate) fn resolve_tag(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-tag requires a data-project attribute")?;
    let project = validate_project(project)?;
    let base_url = resolve_base_url(params)?;

    match params.get("sort").map(String::as_str) {
        None | Some("date") => {}
        Some("semver") => {
            return Err("gitlab-tag: data-sort=\"semver\" is not supported".to_string());
        }
        Some(other) => return Err(format!("gitlab-tag: unsupported data-sort value '{other}'")),
    }

    let url = format!(
        "{base_url}/api/v4/projects/{}/repository/tags?order_by=updated",
        percent_encode(project)
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(tags) = value else {
        return Err("gitlab response was not an array".to_string());
    };
    let first = tags.first().ok_or("no tags found")?;
    first
        .get("name")
        .and_then(Value::as_text)
        .ok_or_else(|| "gitlab tag entry missing name".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: &'static str,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(project: &str) -> HashMap<String, String> {
        HashMap::from([("project".to_string(), project.to_string())])
    }

    #[test]
    fn extracts_the_most_recently_updated_tag_name() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/shields-ops-group%2Ftag-test/repository/tags?order_by=updated",
            body: r#"[{"name": "v1.2.0"}, {"name": "v1.1.0"}]"#,
        };
        let value = resolve_tag(&params("shields-ops-group/tag-test"), &fetcher).unwrap();
        assert_eq!(value, "v1.2.0");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project param")
            }
        }
        assert!(resolve_tag(&HashMap::new(), &Unused).is_err());
        assert!(resolve_tag(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_tag(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn rejects_unsupported_semver_sort() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch when sort is unsupported")
            }
        }
        let mut p = params("shields-ops-group/tag-test");
        p.insert("sort".to_string(), "semver".to_string());
        assert!(resolve_tag(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_no_tags_exist() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/shields-ops-group%2Ftag-test/repository/tags?order_by=updated",
            body: r#"[]"#,
        };
        assert!(resolve_tag(&params("shields-ops-group/tag-test"), &fetcher).is_err());
    }
}
