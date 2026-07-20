use crate::Fetcher;
use crate::json;
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

pub(crate) fn resolve_stars(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-stars requires a data-project attribute")?;
    let project = validate_project(project)?;
    let base_url = resolve_base_url(params)?;

    let url = format!("{base_url}/api/v4/projects/{}", percent_encode(project));
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("star_count")
        .ok_or("gitlab response missing star_count")?;
    count
        .as_text()
        .ok_or_else(|| "star_count was not a plain value".to_string())
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
    fn extracts_star_count_from_a_gitlab_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab",
            body: r#"{"id": 1, "star_count": 4821}"#,
        };
        let value = resolve_stars(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "4821");
    }

    #[test]
    fn uses_a_custom_gitlab_url_when_provided() {
        let mut p = params("shdwmtr/placard");
        p.insert(
            "gitlab-url".to_string(),
            "https://gitlab.example.com/".to_string(),
        );
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.example.com/api/v4/projects/shdwmtr%2Fplacard",
            body: r#"{"star_count": 3}"#,
        };
        let value = resolve_stars(&p, &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project param")
            }
        }
        assert!(resolve_stars(&HashMap::new(), &Unused).is_err());
        assert!(resolve_stars(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_stars(&params("../etc/passwd"), &Unused).is_err());
        assert!(resolve_stars(&params("a//b"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/shdwmtr%2Fplacard",
            body: r#"{"id": 1}"#,
        };
        assert!(resolve_stars(&params("shdwmtr/placard"), &fetcher).is_err());
    }
}
