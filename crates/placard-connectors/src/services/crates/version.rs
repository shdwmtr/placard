use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let crate_name = params
        .get("crate")
        .ok_or("crates-version requires a data-crate attribute")?;
    let crate_name = validate_path_param("crate", crate_name)?;

    let url = format!("https://crates.io/api/v1/crates/{crate_name}?include=versions,downloads");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "crates.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let stable = value
        .get("crate.max_stable_version")
        .and_then(|v| v.as_text())
        .filter(|s| !s.is_empty());
    if let Some(version) = stable {
        return Ok(version);
    }

    value
        .get("crate.max_version")
        .ok_or("crates.io response missing max_version")?
        .as_text()
        .ok_or_else(|| "max_version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://crates.io/api/v1/crates/rustc-serialize?include=versions,downloads"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(crate_name: &str) -> HashMap<String, String> {
        HashMap::from([("crate".to_string(), crate_name.to_string())])
    }

    #[test]
    fn prefers_max_stable_version_when_present() {
        let fetcher = FakeFetcher(
            r#"{"crate": {"max_stable_version": "0.3.24", "max_version": "0.4.0-alpha"}, "versions": [{"num": "0.3.24"}]}"#,
        );
        let value = resolve_version(&params("rustc-serialize"), &fetcher).unwrap();
        assert_eq!(value, "0.3.24");
    }

    #[test]
    fn falls_back_to_max_version_when_no_stable_version() {
        let fetcher = FakeFetcher(
            r#"{"crate": {"max_stable_version": null, "max_version": "0.4.0-alpha"}, "versions": []}"#,
        );
        let value = resolve_version(&params("rustc-serialize"), &fetcher).unwrap();
        assert_eq!(value, "0.4.0-alpha");
    }

    #[test]
    fn requires_crate_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid crate param")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_both_version_fields_are_missing() {
        let fetcher = FakeFetcher(r#"{"crate": {}, "versions": []}"#);
        assert!(resolve_version(&params("rustc-serialize"), &fetcher).is_err());
    }
}
