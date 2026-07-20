use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_last_commit(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("sourceforge-last-commit requires a data-project attribute")?;
    let repo = params
        .get("repo")
        .ok_or("sourceforge-last-commit requires a data-repo attribute")?;
    let project = validate_path_param("project", project)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://sourceforge.net/rest/p/{project}/{repo}/commits");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourceforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let commits = match value.get("commits") {
        Some(Value::Array(items)) => items,
        _ => return Err("sourceforge response missing commits array".to_string()),
    };
    let first = commits
        .first()
        .ok_or("sourceforge response has no commits")?;
    let date = first
        .get("committed_date")
        .ok_or("sourceforge commit missing committed_date")?;
    date.as_text()
        .ok_or_else(|| "committed_date was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://sourceforge.net/rest/p/guitarix/git/commits");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(project: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("project".to_string(), project.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_most_recent_committed_date() {
        let fetcher = FakeFetcher(
            r#"{"commits": [{"committed_date": "2024-01-02T10:00:00Z"}, {"committed_date": "2023-01-01T10:00:00Z"}]}"#,
        );
        let value = resolve_last_commit(&params("guitarix", "git"), &fetcher).unwrap();
        assert_eq!(value, "2024-01-02T10:00:00Z");
    }

    #[test]
    fn requires_project_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_last_commit(&HashMap::new(), &Unused).is_err());
        assert!(resolve_last_commit(&params("guitarix", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_last_commit(&params("../etc", "git"), &Unused).is_err());
    }

    #[test]
    fn errors_when_commits_array_is_empty() {
        let fetcher = FakeFetcher(r#"{"commits": []}"#);
        assert!(resolve_last_commit(&params("guitarix", "git"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_commits_field_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_last_commit(&params("guitarix", "git"), &fetcher).is_err());
    }
}
