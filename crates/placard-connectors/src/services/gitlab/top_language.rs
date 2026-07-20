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

pub(crate) fn resolve_top_language(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-top-language requires a data-project attribute")?;
    let project = validate_project(project)?;
    let base_url = resolve_base_url(params)?;

    let url = format!(
        "{base_url}/api/v4/projects/{}/languages",
        percent_encode(project)
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Object(fields) = value else {
        return Err("gitlab response was not an object".to_string());
    };
    if fields.is_empty() {
        return Err("no languages found".to_string());
    }

    let mut top: Option<(&str, f64)> = None;
    for (name, field_value) in &fields {
        if let Value::Number(percentage) = field_value {
            if top.map(|(_, best)| *percentage > best).unwrap_or(true) {
                top = Some((name.as_str(), *percentage));
            }
        }
    }
    let (_, percentage) = top.ok_or("gitlab response had no numeric language percentages")?;
    Ok(format!("{}%", Value::Number(percentage).as_text().unwrap()))
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
    fn extracts_the_highest_percentage_language() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/languages",
            body: r#"{"Ruby": 66.6, "JavaScript": 25.3, "Vue": 8.1}"#,
        };
        let value = resolve_top_language(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "66.6%");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project param")
            }
        }
        assert!(resolve_top_language(&HashMap::new(), &Unused).is_err());
        assert!(resolve_top_language(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_top_language(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_languages_are_reported() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/languages",
            body: r#"{}"#,
        };
        assert!(resolve_top_language(&params("gitlab-org/gitlab"), &fetcher).is_err());
    }
}
