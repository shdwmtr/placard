use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-license requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-license requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    match value.get("license") {
        None | Some(Value::Null) => Ok("not specified".to_string()),
        Some(license) => {
            let spdx_id = license
                .get("spdx_id")
                .ok_or("github response missing license.spdx_id")?;
            spdx_id
                .as_text()
                .ok_or_else(|| "spdx_id was not a plain value".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.github.com/repos/mashape/apistatus");
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
    fn extracts_the_spdx_id() {
        let fetcher = FakeFetcher(r#"{"id": 1, "license": {"key": "mit", "spdx_id": "MIT"}}"#);
        let value = resolve_license(&params("mashape", "apistatus"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn reports_not_specified_when_license_is_null() {
        let fetcher = FakeFetcher(r#"{"id": 1, "license": null}"#);
        let value = resolve_license(&params("mashape", "apistatus"), &fetcher).unwrap();
        assert_eq!(value, "not specified");
    }

    #[test]
    fn reports_not_specified_when_license_key_is_absent() {
        let fetcher = FakeFetcher(r#"{"id": 1}"#);
        let value = resolve_license(&params("mashape", "apistatus"), &fetcher).unwrap();
        assert_eq!(value, "not specified");
    }

    #[test]
    fn extracts_noassertion() {
        let fetcher = FakeFetcher(r#"{"license": {"key": "other", "spdx_id": "NOASSERTION"}}"#);
        let value = resolve_license(&params("mashape", "apistatus"), &fetcher).unwrap();
        assert_eq!(value, "NOASSERTION");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_license(&HashMap::new(), &Unused).is_err());
        assert!(resolve_license(&params("mashape", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_license(&params("../etc", "apistatus"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_license_object_is_missing_spdx_id() {
        let fetcher = FakeFetcher(r#"{"license": {"key": "mit"}}"#);
        assert!(resolve_license(&params("mashape", "apistatus"), &fetcher).is_err());
    }
}
