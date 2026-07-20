use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_commits_difference(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-commits-difference requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-commits-difference requires a data-repo attribute")?;
    let base = params
        .get("base")
        .ok_or("github-commits-difference requires a data-base attribute")?;
    let head = params
        .get("head")
        .ok_or("github-commits-difference requires a data-head attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let base = validate_path_param("base", base)?;
    let head = validate_path_param("head", head)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/compare/{base}...{head}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("total_commits")
        .ok_or("github response missing total_commits")?;
    count
        .as_text()
        .ok_or_else(|| "total_commits was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/microsoft/vscode/compare/1.60.0...82f2db7"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, base: &str, head: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("base".to_string(), base.to_string()),
            ("head".to_string(), head.to_string()),
        ])
    }

    #[test]
    fn extracts_total_commits_from_a_github_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"status": "ahead", "ahead_by": 12, "behind_by": 0, "total_commits": 12}"#,
        );
        let value = resolve_commits_difference(
            &params("microsoft", "vscode", "1.60.0", "82f2db7"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "12");
    }

    #[test]
    fn requires_owner_repo_base_and_head_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_commits_difference(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_commits_difference(&params("microsoft", "vscode", "1.60.0", ""), &Unused)
                .is_err()
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
            resolve_commits_difference(&params("../etc", "vscode", "1.60.0", "82f2db7"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"status": "ahead"}"#);
        assert!(
            resolve_commits_difference(
                &params("microsoft", "vscode", "1.60.0", "82f2db7"),
                &fetcher
            )
            .is_err()
        );
    }
}
