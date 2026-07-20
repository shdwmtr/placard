use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_commit_count(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("sourceforge-commit-count requires a data-project attribute")?;
    let repo = params
        .get("repo")
        .ok_or("sourceforge-commit-count requires a data-repo attribute")?;
    let project = validate_path_param("project", project)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://sourceforge.net/rest/p/{project}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourceforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("commit_count")
        .ok_or("sourceforge response missing commit_count")?;
    count
        .as_text()
        .ok_or_else(|| "commit_count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://sourceforge.net/rest/p/guitarix/git");
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
    fn extracts_commit_count_from_a_sourceforge_shaped_response() {
        let fetcher = FakeFetcher(r#"{"commit_count": 4821}"#);
        let value = resolve_commit_count(&params("guitarix", "git"), &fetcher).unwrap();
        assert_eq!(value, "4821");
    }

    #[test]
    fn requires_project_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_commit_count(&HashMap::new(), &Unused).is_err());
        assert!(resolve_commit_count(&params("guitarix", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_commit_count(&params("../etc", "git"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_commit_count(&params("guitarix", "git"), &fetcher).is_err());
    }
}
