use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_checks_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-checks-status requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-checks-status requires a data-repo attribute")?;
    let git_ref = params
        .get("ref")
        .ok_or("github-checks-status requires a data-ref attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let git_ref = validate_path_param("ref", git_ref)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/commits/{git_ref}/status");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let state = value.get("state").ok_or("github response missing state")?;
    state
        .as_text()
        .ok_or_else(|| "state was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/badges/shields/commits/master/status"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, git_ref: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("ref".to_string(), git_ref.to_string()),
        ])
    }

    #[test]
    fn extracts_state_from_a_github_shaped_response() {
        let fetcher = FakeFetcher(r#"{"state": "success", "statuses": []}"#);
        let value =
            resolve_checks_status(&params("badges", "shields", "master"), &fetcher).unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn requires_owner_repo_and_ref_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_checks_status(&HashMap::new(), &Unused).is_err());
        assert!(resolve_checks_status(&params("badges", "shields", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_checks_status(&params("../etc", "shields", "master"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_state_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"statuses": []}"#);
        assert!(resolve_checks_status(&params("badges", "shields", "master"), &fetcher).is_err());
    }
}
