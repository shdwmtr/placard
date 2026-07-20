use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("bower-license requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://libraries.io/api/bower/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "libraries.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let licenses = value
        .get("normalized_licenses")
        .ok_or("libraries.io response missing normalized_licenses")?;
    let Value::Array(items) = licenses else {
        return Err("libraries.io response 'normalized_licenses' was not an array".to_string());
    };

    let names: Vec<String> = items.iter().filter_map(|item| item.as_text()).collect();
    if names.is_empty() {
        return Ok("missing".to_string());
    }
    Ok(names.join(", "))
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
    fn extracts_a_single_license() {
        let fetcher = FakeFetcher(
            r#"{"normalized_licenses": ["MIT"], "latest_release_number": "5.3.3", "latest_stable_release_number": "5.3.3"}"#,
        );
        let value = resolve_license(&params("bootstrap"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn joins_multiple_licenses_with_a_comma() {
        let fetcher = FakeFetcher(
            r#"{"normalized_licenses": ["MIT", "ISC"], "latest_release_number": "1.0.0", "latest_stable_release_number": "1.0.0"}"#,
        );
        let value = resolve_license(&params("bootstrap"), &fetcher).unwrap();
        assert_eq!(value, "MIT, ISC");
    }

    #[test]
    fn reports_missing_when_no_licenses_are_declared() {
        let fetcher = FakeFetcher(
            r#"{"normalized_licenses": [], "latest_release_number": null, "latest_stable_release_number": null}"#,
        );
        let value = resolve_license(&params("bootstrap"), &fetcher).unwrap();
        assert_eq!(value, "missing");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_license(&HashMap::new(), &Unused).is_err());
        assert!(resolve_license(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_license(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_normalized_licenses_is_missing() {
        let fetcher = FakeFetcher(r#"{"latest_release_number": "1.0.0"}"#);
        assert!(resolve_license(&params("bootstrap"), &fetcher).is_err());
    }
}
