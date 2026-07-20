use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_commit_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-commit-status requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-commit-status requires a data-repo attribute")?;
    let branch = params
        .get("branch")
        .ok_or("github-commit-status requires a data-branch attribute")?;
    let commit = params
        .get("commit")
        .ok_or("github-commit-status requires a data-commit attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = validate_path_param("branch", branch)?;
    let commit = validate_path_param("commit", commit)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/compare/{branch}...{commit}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let status = value
        .get("status")
        .ok_or("github response missing status")?;
    status
        .as_text()
        .ok_or_else(|| "status was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/badges/shields/compare/master...5d4ab86b1b5ddfb3c4a70a70bd19932c52603b8c"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, branch: &str, commit: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("branch".to_string(), branch.to_string()),
            ("commit".to_string(), commit.to_string()),
        ])
    }

    #[test]
    fn extracts_status_from_a_github_shaped_response() {
        let fetcher = FakeFetcher(r#"{"status": "behind", "ahead_by": 0, "behind_by": 3}"#);
        let value = resolve_commit_status(
            &params(
                "badges",
                "shields",
                "master",
                "5d4ab86b1b5ddfb3c4a70a70bd19932c52603b8c",
            ),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "behind");
    }

    #[test]
    fn requires_owner_repo_branch_and_commit_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_commit_status(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_commit_status(&params("badges", "shields", "master", ""), &Unused).is_err()
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
            resolve_commit_status(&params("../etc", "shields", "master", "5d4ab8"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"ahead_by": 0}"#);
        assert!(
            resolve_commit_status(
                &params(
                    "badges",
                    "shields",
                    "master",
                    "5d4ab86b1b5ddfb3c4a70a70bd19932c52603b8c"
                ),
                &fetcher
            )
            .is_err()
        );
    }
}
