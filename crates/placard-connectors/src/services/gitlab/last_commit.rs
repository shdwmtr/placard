use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_project(project: &str) -> Result<&str, String> {
    if project.is_empty() {
        return Err("'project' parameter must not be empty".to_string());
    }
    if project.starts_with('/')
        || project.ends_with('/')
        || project.contains("//")
        || project.contains("..")
    {
        return Err("'project' parameter is not a valid GitLab project path".to_string());
    }
    if !project
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        return Err("'project' parameter contains disallowed characters".to_string());
    }
    Ok(project)
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn base_url(params: &HashMap<String, String>) -> String {
    match params.get("gitlab-url") {
        Some(v) if !v.is_empty() => v.trim_end_matches('/').to_string(),
        _ => "https://gitlab.com".to_string(),
    }
}

pub(crate) fn resolve_last_commit(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-last-commit requires a data-project attribute")?;
    let project = validate_project(project)?;
    let base = base_url(params);

    let mut url = format!(
        "{base}/api/v4/projects/{}/repository/commits?",
        percent_encode(project)
    );
    if let Some(reference) = params.get("ref") {
        url.push_str("ref_name=");
        url.push_str(&percent_encode(reference));
        url.push('&');
    }
    if let Some(path) = params.get("path") {
        url.push_str("path=");
        url.push_str(&percent_encode(path));
        url.push('&');
    }
    url.push_str("per_page=1");

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(items) = value else {
        return Err("gitlab response was not an array".to_string());
    };
    let commit = items.first().ok_or("no commits found")?;
    let committed_date = commit
        .get("committed_date")
        .ok_or("gitlab response missing committed_date")?;
    committed_date
        .as_text()
        .ok_or_else(|| "committed_date was not a plain value".to_string())
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
    fn extracts_committed_date_from_a_gitlab_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/commits?per_page=1",
            body: r#"[{"id": "abc123", "committed_date": "2024-05-01T12:00:00.000Z"}]"#,
        };
        let value = resolve_last_commit(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "2024-05-01T12:00:00.000Z");
    }

    #[test]
    fn appends_ref_and_path_to_the_query_when_provided() {
        let mut p = params("gitlab-org/gitlab");
        p.insert("ref".to_string(), "main".to_string());
        p.insert("path".to_string(), "README.md".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/commits?ref_name=main&path=README.md&per_page=1",
            body: r#"[{"committed_date": "2024-01-01T00:00:00.000Z"}]"#,
        };
        let value = resolve_last_commit(&p, &fetcher).unwrap();
        assert_eq!(value, "2024-01-01T00:00:00.000Z");
    }

    #[test]
    fn errors_when_there_are_no_commits() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/commits?per_page=1",
            body: r#"[]"#,
        };
        assert!(resolve_last_commit(&params("gitlab-org/gitlab"), &fetcher).is_err());
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project")
            }
        }
        assert!(resolve_last_commit(&HashMap::new(), &Unused).is_err());
        assert!(resolve_last_commit(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_project_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project")
            }
        }
        assert!(resolve_last_commit(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_committed_date_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/commits?per_page=1",
            body: r#"[{"id": "abc123"}]"#,
        };
        assert!(resolve_last_commit(&params("gitlab-org/gitlab"), &fetcher).is_err());
    }
}
