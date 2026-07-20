use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_commits_since(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-commits-since requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-commits-since requires a data-repo attribute")?;
    let version = params
        .get("version")
        .ok_or("github-commits-since requires a data-version attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let version = validate_path_param("version", version)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };

    let url = format!("https://api.github.com/repos/{owner}/{repo}/compare/{version}...{branch}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("ahead_by")
        .ok_or("github response missing ahead_by")?;
    count
        .as_text()
        .ok_or_else(|| "ahead_by was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/SubtitleEdit/subtitleedit/compare/3.4.7...HEAD"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, version: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("version".to_string(), version.to_string()),
        ])
    }

    #[test]
    fn extracts_ahead_by_from_a_github_shaped_response() {
        let fetcher = FakeFetcher(r#"{"status": "ahead", "ahead_by": 42, "behind_by": 0}"#);
        let value =
            resolve_commits_since(&params("SubtitleEdit", "subtitleedit", "3.4.7"), &fetcher)
                .unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn uses_the_branch_param_when_present() {
        struct FakeBranchFetcher;
        impl Fetcher for FakeBranchFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://api.github.com/repos/SubtitleEdit/subtitleedit/compare/3.4.7...develop"
                );
                Ok(r#"{"ahead_by": 7}"#.as_bytes().to_vec())
            }
        }
        let mut p = params("SubtitleEdit", "subtitleedit", "3.4.7");
        p.insert("branch".to_string(), "develop".to_string());
        let value = resolve_commits_since(&p, &FakeBranchFetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn requires_owner_repo_and_version_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_commits_since(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_commits_since(&params("SubtitleEdit", "subtitleedit", ""), &Unused).is_err()
        );
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(
            resolve_commits_since(&params("../etc", "subtitleedit", "3.4.7"), &Unused).is_err()
        );
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"status": "ahead"}"#);
        assert!(
            resolve_commits_since(&params("SubtitleEdit", "subtitleedit", "3.4.7"), &fetcher)
                .is_err()
        );
    }
}
