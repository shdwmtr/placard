use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_all_contributors(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-all-contributors requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-all-contributors requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };

    let url =
        format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/.all-contributorsrc");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    match value.get("contributors") {
        Some(Value::Array(items)) => Ok(items.len().to_string()),
        _ => Err("all-contributorsrc response missing contributors array".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/all-contributors/all-contributors/HEAD/.all-contributorsrc"
            );
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
    fn counts_the_contributors_array() {
        let fetcher =
            FakeFetcher(r#"{"contributors": [{"login": "a"}, {"login": "b"}, {"login": "c"}]}"#);
        let value =
            resolve_all_contributors(&params("all-contributors", "all-contributors"), &fetcher)
                .unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_all_contributors(&HashMap::new(), &Unused).is_err());
        assert!(resolve_all_contributors(&params("all-contributors", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_all_contributors(&params("../etc", "all-contributors"), &Unused).is_err());
    }

    #[test]
    fn errors_when_contributors_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"projectName": "all-contributors"}"#);
        assert!(
            resolve_all_contributors(&params("all-contributors", "all-contributors"), &fetcher)
                .is_err()
        );
    }
}
