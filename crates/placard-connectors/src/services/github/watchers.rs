use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_watchers(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-watchers requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-watchers requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("subscribers_count")
        .ok_or("github response missing subscribers_count")?;
    count
        .as_text()
        .ok_or_else(|| "subscribers_count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.github.com/repos/badges/shields");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_subscribers_count_from_a_github_shaped_response() {
        let fetcher = FakeFetcher(r#"{"id": 1, "name": "shields", "subscribers_count": 421}"#);
        let value = resolve_watchers(&params("badges", "shields"), &fetcher).unwrap();
        assert_eq!(value, "421");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_watchers(&HashMap::new(), &Unused).is_err());
        assert!(resolve_watchers(&params("badges", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_watchers(&params("../etc", "shields"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"id": 1}"#);
        assert!(resolve_watchers(&params("badges", "shields"), &fetcher).is_err());
    }
}
