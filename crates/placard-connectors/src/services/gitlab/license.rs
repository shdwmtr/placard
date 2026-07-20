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

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-license requires a data-project attribute")?;
    let project = validate_project(project)?;
    let base = base_url(params);

    let url = format!(
        "{base}/api/v4/projects/{}?license=1",
        percent_encode_project(project)
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    match value.get("license") {
        None | Some(Value::Null) => Ok("not specified".to_string()),
        Some(license) => {
            let name = license
                .get("name")
                .ok_or("gitlab response missing license.name")?;
            name.as_text()
                .ok_or_else(|| "license.name was not a plain value".to_string())
        }
    }
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
    fn extracts_license_name_from_a_gitlab_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab?license=1",
            body: r#"{"license": {"key": "mit", "name": "MIT License"}}"#,
        };
        let value = resolve_license(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "MIT License");
    }

    #[test]
    fn returns_not_specified_when_license_is_null() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab?license=1",
            body: r#"{"license": null}"#,
        };
        let value = resolve_license(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "not specified");
    }

    #[test]
    fn returns_not_specified_when_license_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab?license=1",
            body: r#"{"id": 1}"#,
        };
        let value = resolve_license(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "not specified");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project")
            }
        }
        assert!(resolve_license(&HashMap::new(), &Unused).is_err());
        assert!(resolve_license(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_project_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project")
            }
        }
        assert!(resolve_license(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_license_name_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab?license=1",
            body: r#"{"license": {"key": "mit"}}"#,
        };
        assert!(resolve_license(&params("gitlab-org/gitlab"), &fetcher).is_err());
    }
}
