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

fn counts_field(variant: &str) -> Result<&'static str, String> {
    let state = variant.split('-').next().unwrap_or(variant);
    match state {
        "all" => Ok("all"),
        "open" => Ok("opened"),
        "closed" => Ok("closed"),
        _ => Err(format!(
            "'variant' parameter '{variant}' is not one of all, all-raw, open, open-raw, closed, closed-raw"
        )),
    }
}

pub(crate) fn resolve_issues(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-issues requires a data-project attribute")?;
    let project = validate_project(project)?;
    let variant = params
        .get("variant")
        .ok_or("gitlab-issues requires a data-variant attribute")?;
    let field = counts_field(variant)?;
    let base = base_url(params);

    let mut url = format!(
        "{base}/api/v4/projects/{}/issues_statistics",
        percent_encode(project)
    );
    if let Some(labels) = params.get("labels") {
        url.push_str("?labels=");
        url.push_str(&percent_encode(labels));
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get(&format!("statistics.counts.{field}"))
        .ok_or_else(|| format!("gitlab response missing statistics.counts.{field}"))?;
    count
        .as_text()
        .ok_or_else(|| format!("statistics.counts.{field} was not a plain value"))
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

    fn params(project: &str, variant: &str) -> HashMap<String, String> {
        HashMap::from([
            ("project".to_string(), project.to_string()),
            ("variant".to_string(), variant.to_string()),
        ])
    }

    #[test]
    fn extracts_opened_count_for_the_open_variant() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/issues_statistics",
            body: r#"{"statistics": {"counts": {"all": 100, "closed": 60, "opened": 40}}}"#,
        };
        let value = resolve_issues(&params("gitlab-org/gitlab", "open"), &fetcher).unwrap();
        assert_eq!(value, "40");
    }

    #[test]
    fn strips_the_raw_suffix_from_the_variant() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/issues_statistics",
            body: r#"{"statistics": {"counts": {"all": 100, "closed": 60, "opened": 40}}}"#,
        };
        let value = resolve_issues(&params("gitlab-org/gitlab", "closed-raw"), &fetcher).unwrap();
        assert_eq!(value, "60");
    }

    #[test]
    fn appends_labels_to_the_query_when_provided() {
        let mut p = params("gitlab-org/gitlab", "all");
        p.insert("labels".to_string(), "bug,critical".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/issues_statistics?labels=bug%2Ccritical",
            body: r#"{"statistics": {"counts": {"all": 5, "closed": 2, "opened": 3}}}"#,
        };
        let value = resolve_issues(&p, &fetcher).unwrap();
        assert_eq!(value, "5");
    }

    #[test]
    fn requires_project_and_variant_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_issues(&HashMap::new(), &Unused).is_err());
        assert!(resolve_issues(&params("gitlab-org/gitlab", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_variant_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid variant")
            }
        }
        assert!(resolve_issues(&params("gitlab-org/gitlab", "weird"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_project_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project")
            }
        }
        assert!(resolve_issues(&params("../etc/passwd", "all"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/issues_statistics",
            body: r#"{"statistics": {"counts": {}}}"#,
        };
        assert!(resolve_issues(&params("gitlab-org/gitlab", "all"), &fetcher).is_err());
    }
}
