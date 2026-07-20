use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_docsrs(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let crate_name = params
        .get("crate")
        .ok_or("docsrs requires a data-crate attribute")?;
    let crate_name = validate_path_param("crate", crate_name)?;
    let version = params
        .get("version")
        .map(String::as_str)
        .unwrap_or("latest");
    let version = validate_path_param("version", version)?;

    let url = format!("https://docs.rs/crate/{crate_name}/{version}/status.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "docs.rs response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let doc_status = value
        .get("doc_status")
        .ok_or("docs.rs response missing doc_status")?;
    match doc_status {
        Value::Bool(true) => Ok("passing".to_string()),
        Value::Bool(false) => Ok("failing".to_string()),
        _ => Err("doc_status was not a boolean".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://docs.rs/crate/regex/latest/status.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(crate_name: &str) -> HashMap<String, String> {
        HashMap::from([("crate".to_string(), crate_name.to_string())])
    }

    #[test]
    fn reports_passing_when_doc_status_is_true() {
        let fetcher = FakeFetcher(r#"{"doc_status": true}"#);
        let value = resolve_docsrs(&params("regex"), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn reports_failing_when_doc_status_is_false() {
        let fetcher = FakeFetcher(r#"{"doc_status": false}"#);
        let value = resolve_docsrs(&params("regex"), &fetcher).unwrap();
        assert_eq!(value, "failing");
    }

    #[test]
    fn uses_the_version_param_in_the_url() {
        struct FetcherAssertingVersion;
        impl Fetcher for FetcherAssertingVersion {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(url, "https://docs.rs/crate/regex/1.5.4/status.json");
                Ok(r#"{"doc_status": true}"#.as_bytes().to_vec())
            }
        }
        let mut p = params("regex");
        p.insert("version".to_string(), "1.5.4".to_string());
        let value = resolve_docsrs(&p, &FetcherAssertingVersion).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn requires_crate_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_docsrs(&HashMap::new(), &Unused).is_err());
        assert!(resolve_docsrs(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_docsrs(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_docsrs(&params("regex"), &fetcher).is_err());
    }
}
