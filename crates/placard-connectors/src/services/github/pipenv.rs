use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_pipenv(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-pipenv requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-pipenv requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };

    let url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/Pipfile.lock");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("_meta.requires.python_version")
        .ok_or("Pipfile.lock missing _meta.requires.python_version")?;
    version
        .as_text()
        .ok_or_else(|| "python_version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/metabolize/rq-dashboard-on-heroku/HEAD/Pipfile.lock"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "metabolize".to_string()),
            ("repo".to_string(), "rq-dashboard-on-heroku".to_string()),
        ])
    }

    #[test]
    fn extracts_the_python_version() {
        let fetcher = FakeFetcher(r#"{"_meta": {"requires": {"python_version": "3.11"}}}"#);
        let value = resolve_pipenv(&params(), &fetcher).unwrap();
        assert_eq!(value, "3.11");
    }

    #[test]
    fn uses_a_custom_branch_when_given() {
        struct BranchFetcher;
        impl Fetcher for BranchFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://raw.githubusercontent.com/metabolize/rq-dashboard-on-heroku/main/Pipfile.lock"
                );
                Ok(br#"{"_meta": {"requires": {"python_version": "3.12"}}}"#.to_vec())
            }
        }
        let mut p = params();
        p.insert("branch".to_string(), "main".to_string());
        let value = resolve_pipenv(&p, &BranchFetcher).unwrap();
        assert_eq!(value, "3.12");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_pipenv(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("repo".to_string(), String::new());
        assert!(resolve_pipenv(&p, &Unused).is_err());
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
        assert!(resolve_pipenv(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_python_version_is_missing() {
        let fetcher = FakeFetcher(r#"{"_meta": {"requires": {}}}"#);
        assert!(resolve_pipenv(&params(), &fetcher).is_err());
    }
}
