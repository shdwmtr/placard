use crate::Fetcher;
use crate::json;
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

fn percent_encode_project(project: &str) -> String {
    let mut out = String::with_capacity(project.len());
    for byte in project.bytes() {
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

pub(crate) fn resolve_forks(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-forks requires a data-project attribute")?;
    let project = validate_project(project)?;
    let base = base_url(params);

    let url = format!("{base}/api/v4/projects/{}", percent_encode_project(project));
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("forks_count")
        .ok_or("gitlab response missing forks_count")?;
    count
        .as_text()
        .ok_or_else(|| "forks_count was not a plain value".to_string())
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
    fn extracts_forks_count_from_a_gitlab_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab",
            body: r#"{"id": 1, "forks_count": 4821}"#,
        };
        let value = resolve_forks(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "4821");
    }

    #[test]
    fn uses_a_custom_gitlab_url_when_provided() {
        let mut p = params("owner/repo");
        p.insert(
            "gitlab-url".to_string(),
            "https://gitlab.example.com/".to_string(),
        );
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.example.com/api/v4/projects/owner%2Frepo",
            body: r#"{"forks_count": 2}"#,
        };
        let value = resolve_forks(&p, &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project")
            }
        }
        assert!(resolve_forks(&HashMap::new(), &Unused).is_err());
        assert!(resolve_forks(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_project_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project")
            }
        }
        assert!(resolve_forks(&params("../etc/passwd"), &Unused).is_err());
        assert!(resolve_forks(&params("/leading-slash"), &Unused).is_err());
        assert!(resolve_forks(&params("a?b=c"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab",
            body: r#"{"id": 1}"#,
        };
        assert!(resolve_forks(&params("gitlab-org/gitlab"), &fetcher).is_err());
    }
}
