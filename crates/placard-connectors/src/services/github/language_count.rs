use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_language_count(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-language-count requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-language-count requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/languages");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Object(fields) = value else {
        return Err("github response was not a JSON object".to_string());
    };
    Ok(fields.len().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.github.com/repos/badges/shields/languages");
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
    fn counts_the_languages_in_the_response_object() {
        let fetcher = FakeFetcher(r#"{"Python": 39624, "Shell": 104, "JavaScript": 8000}"#);
        let value = resolve_language_count(&params("badges", "shields"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn handles_an_empty_repo() {
        let fetcher = FakeFetcher(r#"{}"#);
        let value = resolve_language_count(&params("badges", "shields"), &fetcher).unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_language_count(&HashMap::new(), &Unused).is_err());
        assert!(resolve_language_count(&params("badges", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_language_count(&params("../etc", "shields"), &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_not_an_object() {
        let fetcher = FakeFetcher(r#"[1, 2, 3]"#);
        assert!(resolve_language_count(&params("badges", "shields"), &fetcher).is_err());
    }
}
