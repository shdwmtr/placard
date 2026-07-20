use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_repo_size(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-repo-size requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-repo-size requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let size = value.get("size").ok_or("github response missing size")?;
    size.as_text()
        .ok_or_else(|| "size was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.github.com/repos/atom/atom");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "atom".to_string()),
            ("repo".to_string(), "atom".to_string()),
        ])
    }

    #[test]
    fn extracts_the_size_field_in_kib() {
        let fetcher = FakeFetcher(r#"{"id": 1, "size": 314159}"#);
        let value = resolve_repo_size(&params(), &fetcher).unwrap();
        assert_eq!(value, "314159");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_repo_size(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("repo".to_string(), String::new());
        assert!(resolve_repo_size(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params();
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_repo_size(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_size_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"id": 1}"#);
        assert!(resolve_repo_size(&params(), &fetcher).is_err());
    }
}
