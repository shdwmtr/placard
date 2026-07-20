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
        .ok_or("hexpm-version requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://hex.pm/api/packages/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "hexpm response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    if let Some(stable) = value.get("latest_stable_version").and_then(|v| v.as_text()) {
        return Ok(stable);
    }
    value
        .get("latest_version")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "hexpm response missing latest_version".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://hex.pm/api/packages/plug");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn prefers_latest_stable_version_when_present() {
        let fetcher =
            FakeFetcher(r#"{"latest_stable_version": "1.14.0", "latest_version": "1.15.0-rc.0"}"#);
        let value = resolve_version(&params("plug"), &fetcher).unwrap();
        assert_eq!(value, "1.14.0");
    }

    #[test]
    fn falls_back_to_latest_version_when_stable_is_null() {
        let fetcher =
            FakeFetcher(r#"{"latest_stable_version": null, "latest_version": "1.15.0-rc.0"}"#);
        let value = resolve_version(&params("plug"), &fetcher).unwrap();
        assert_eq!(value, "1.15.0-rc.0");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
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
        let fetcher = FakeFetcher(r#"{"downloads": {}}"#);
        assert!(resolve_version(&params("plug"), &fetcher).is_err());
    }
}
