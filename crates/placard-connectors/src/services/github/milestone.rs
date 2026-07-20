use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_milestone(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-milestone requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-milestone requires a data-repo attribute")?;
    let variant = params
        .get("variant")
        .ok_or("github-milestone requires a data-variant attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    if !matches!(variant.as_str(), "open" | "closed" | "all") {
        return Err(format!(
            "'variant' parameter '{variant}' is not one of open, closed, all"
        ));
    }

    let url = format!("https://api.github.com/repos/{owner}/{repo}/milestones?state={variant}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    match value {
        Value::Array(items) => Ok(items.len().to_string()),
        _ => Err("github response was not a JSON array".to_string()),
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
                "https://api.github.com/repos/badges/shields/milestones?state=open"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(variant: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "badges".to_string()),
            ("repo".to_string(), "shields".to_string()),
            ("variant".to_string(), variant.to_string()),
        ])
    }

    #[test]
    fn counts_the_milestones_array() {
        let fetcher = FakeFetcher(r#"[{"state": "open"}, {"state": "open"}]"#);
        let value = resolve_milestone(&params("open"), &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn requires_owner_repo_and_variant_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_milestone(&HashMap::new(), &Unused).is_err());
        let mut p = params("open");
        p.insert("repo".to_string(), String::new());
        assert!(resolve_milestone(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_an_unknown_variant() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid variant")
            }
        }
        assert!(resolve_milestone(&params("bogus"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params("open");
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_milestone(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_not_an_array() {
        let fetcher = FakeFetcher(r#"{"message": "not found"}"#);
        assert!(resolve_milestone(&params("open"), &fetcher).is_err());
    }
}
