use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_package_json(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-package-json requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-package-json requires a data-repo attribute")?;
    let key = params
        .get("key")
        .ok_or("github-package-json requires a data-key attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    if key.is_empty() {
        return Err("'key' parameter must not be empty".to_string());
    }
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };

    let url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/package.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let field = value
        .get(key)
        .ok_or_else(|| format!("package.json missing {key}"))?;
    field
        .as_text()
        .ok_or_else(|| format!("{key} was not a plain value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/developit/microbundle/HEAD/package.json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(key: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "developit".to_string()),
            ("repo".to_string(), "microbundle".to_string()),
            ("key".to_string(), key.to_string()),
        ])
    }

    #[test]
    fn extracts_the_requested_key() {
        let fetcher = FakeFetcher(r#"{"name": "microbundle", "version": "0.15.1"}"#);
        let value = resolve_package_json(&params("version"), &fetcher).unwrap();
        assert_eq!(value, "0.15.1");
    }

    #[test]
    fn uses_a_custom_branch_when_given() {
        struct BranchFetcher;
        impl Fetcher for BranchFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://raw.githubusercontent.com/developit/microbundle/main/package.json"
                );
                Ok(br#"{"version": "1.0.0"}"#.to_vec())
            }
        }
        let mut p = params("version");
        p.insert("branch".to_string(), "main".to_string());
        let value = resolve_package_json(&p, &BranchFetcher).unwrap();
        assert_eq!(value, "1.0.0");
    }

    #[test]
    fn requires_owner_repo_and_key_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_package_json(&HashMap::new(), &Unused).is_err());
        let mut p = params("");
        p.insert("key".to_string(), String::new());
        assert!(resolve_package_json(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params("version");
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_package_json(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_key_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "microbundle"}"#);
        assert!(resolve_package_json(&params("version"), &fetcher).is_err());
    }
}
