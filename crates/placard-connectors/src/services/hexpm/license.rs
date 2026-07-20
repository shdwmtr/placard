use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("hexpm-license requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://hex.pm/api/packages/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "hexpm response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let licenses = value
        .get("meta.licenses")
        .ok_or("hexpm response missing meta.licenses")?;
    let json::Value::Array(items) = licenses else {
        return Err("meta.licenses was not an array".to_string());
    };
    if items.is_empty() {
        return Ok("Unknown".to_string());
    }
    let parts: Vec<String> = items.iter().filter_map(|v| v.as_text()).collect();
    if parts.is_empty() {
        return Err("meta.licenses contained no plain values".to_string());
    }
    Ok(parts.join(", "))
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
    fn joins_multiple_licenses() {
        let fetcher = FakeFetcher(r#"{"meta": {"licenses": ["Apache-2.0", "MIT"]}}"#);
        let value = resolve_license(&params("plug"), &fetcher).unwrap();
        assert_eq!(value, "Apache-2.0, MIT");
    }

    #[test]
    fn returns_unknown_when_licenses_is_empty() {
        let fetcher = FakeFetcher(r#"{"meta": {"licenses": []}}"#);
        let value = resolve_license(&params("plug"), &fetcher).unwrap();
        assert_eq!(value, "Unknown");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
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
    fn errors_when_the_licenses_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"meta": {}}"#);
        assert!(resolve_license(&params("plug"), &fetcher).is_err());
    }
}
