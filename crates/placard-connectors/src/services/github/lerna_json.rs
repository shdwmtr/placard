use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_lerna_json(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-lerna-json requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-lerna-json requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };

    let url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/lerna.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("lerna.json response missing version")?;
    version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/babel/babel/HEAD/lerna.json"
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
    fn extracts_the_version_field() {
        let fetcher = FakeFetcher(r#"{"version": "7.20.0", "packages": ["packages/*"]}"#);
        let value = resolve_lerna_json(&params("babel", "babel"), &fetcher).unwrap();
        assert_eq!(value, "7.20.0");
    }

    #[test]
    fn extracts_an_independent_version() {
        let fetcher = FakeFetcher(r#"{"version": "independent"}"#);
        let value = resolve_lerna_json(&params("babel", "babel"), &fetcher).unwrap();
        assert_eq!(value, "independent");
    }

    #[test]
    fn uses_the_given_branch_when_provided() {
        struct BranchFetcher;
        impl Fetcher for BranchFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://raw.githubusercontent.com/babel/babel/main/lerna.json"
                );
                Ok(br#"{"version": "8.0.0"}"#.to_vec())
            }
        }
        let mut p = params("babel", "babel");
        p.insert("branch".to_string(), "main".to_string());
        let value = resolve_lerna_json(&p, &BranchFetcher).unwrap();
        assert_eq!(value, "8.0.0");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_lerna_json(&HashMap::new(), &Unused).is_err());
        assert!(resolve_lerna_json(&params("babel", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_lerna_json(&params("../etc", "babel"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"packages": ["packages/*"]}"#);
        assert!(resolve_lerna_json(&params("babel", "babel"), &fetcher).is_err());
    }
}
