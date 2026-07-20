use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("bower-version requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://libraries.io/api/bower/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "libraries.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let version = value
        .get("latest_release_number")
        .ok_or("libraries.io response missing latest_release_number")?;
    version
        .as_text()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "no releases".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://libraries.io/api/bower/bootstrap");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_the_latest_release_number() {
        let fetcher = FakeFetcher(
            r#"{"normalized_licenses": ["MIT"], "latest_release_number": "5.3.3", "latest_stable_release_number": "5.3.3"}"#,
        );
        let value = resolve_version(&params("bootstrap"), &fetcher).unwrap();
        assert_eq!(value, "5.3.3");
    }

    #[test]
    fn errors_when_there_are_no_releases() {
        let fetcher = FakeFetcher(
            r#"{"normalized_licenses": [], "latest_release_number": null, "latest_stable_release_number": null}"#,
        );
        assert!(resolve_version(&params("bootstrap"), &fetcher).is_err());
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
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
    fn errors_when_latest_release_number_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"normalized_licenses": ["MIT"]}"#);
        assert!(resolve_version(&params("bootstrap"), &fetcher).is_err());
    }
}
