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
        .ok_or("aur-license requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://aur.archlinux.org/rpc?v=5&type=info&arg={package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "aur response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let results = value.get("results").ok_or("aur response missing results")?;
    let Value::Array(results) = results else {
        return Err("aur response's results field was not an array".to_string());
    };
    let first = results.first().ok_or("aur package not found")?;
    let license = first.get("License").ok_or("aur result missing License")?;
    let Value::Array(licenses) = license else {
        return Err("aur package has no license specified".to_string());
    };
    if licenses.is_empty() {
        return Err("aur package has no license specified".to_string());
    }
    let names: Vec<String> = licenses
        .iter()
        .map(|v| {
            v.as_text()
                .ok_or_else(|| "license entry was not a plain value".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    const URL: &str = "https://aur.archlinux.org/rpc?v=5&type=info&arg=android-studio";

    #[test]
    fn extracts_license_from_an_aur_shaped_response() {
        let fetcher = FakeFetcher(
            URL,
            r#"{"resultcount": 1, "results": [{"License": ["custom:eula"]}]}"#,
        );
        let value = resolve_license(&params("android-studio"), &fetcher).unwrap();
        assert_eq!(value, "custom:eula");
    }

    #[test]
    fn joins_multiple_licenses() {
        let fetcher = FakeFetcher(
            URL,
            r#"{"resultcount": 1, "results": [{"License": ["GPL", "MIT"]}]}"#,
        );
        let value = resolve_license(&params("android-studio"), &fetcher).unwrap();
        assert_eq!(value, "GPL, MIT");
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
        assert!(resolve_license(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_license_is_missing_or_empty() {
        let fetcher = FakeFetcher(URL, r#"{"resultcount": 1, "results": [{"License": null}]}"#);
        assert!(resolve_license(&params("android-studio"), &fetcher).is_err());

        let fetcher = FakeFetcher(URL, r#"{"resultcount": 1, "results": [{"License": []}]}"#);
        assert!(resolve_license(&params("android-studio"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_package_not_found() {
        let fetcher = FakeFetcher(
            "https://aur.archlinux.org/rpc?v=5&type=info&arg=nonexistent-pkg",
            r#"{"resultcount": 0, "results": []}"#,
        );
        assert!(resolve_license(&params("nonexistent-pkg"), &fetcher).is_err());
    }
}
