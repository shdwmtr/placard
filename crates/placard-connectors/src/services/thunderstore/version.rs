use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let namespace = params
        .get("namespace")
        .ok_or("thunderstore-version requires a data-namespace attribute")?;
    let namespace = validate_path_param("namespace", namespace)?;
    let package_name = params
        .get("package-name")
        .ok_or("thunderstore-version requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://thunderstore.io/api/v1/package-metrics/{namespace}/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "thunderstore response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("latest_version")
        .ok_or("thunderstore response missing latest_version")?;
    version
        .as_text()
        .ok_or_else(|| "latest_version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://thunderstore.io/api/v1/package-metrics/notnotnotswipez/MoreCompany"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(namespace: &str, package_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("namespace".to_string(), namespace.to_string()),
            ("package-name".to_string(), package_name.to_string()),
        ])
    }

    #[test]
    fn extracts_latest_version_from_package_metrics() {
        let fetcher =
            FakeFetcher(r#"{"downloads": 15234, "rating_score": 42, "latest_version": "1.2.3"}"#);
        let value = resolve_version(&params("notnotnotswipez", "MoreCompany"), &fetcher).unwrap();
        assert_eq!(value, "1.2.3");
    }

    #[test]
    fn requires_namespace_and_package_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("notnotnotswipez", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("notnotnotswipez", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"downloads": 15234, "rating_score": 42}"#);
        assert!(resolve_version(&params("notnotnotswipez", "MoreCompany"), &fetcher).is_err());
    }
}
