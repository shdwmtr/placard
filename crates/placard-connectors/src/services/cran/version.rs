use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("cran-version requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("http://crandb.r-pkg.org/{package}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "cran response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("Version")
        .ok_or("cran response missing Version")?;
    version
        .as_text()
        .ok_or_else(|| "Version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "http://crandb.r-pkg.org/devtools");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_version_from_a_cran_shaped_response() {
        let fetcher = FakeFetcher(r#"{"License": "MIT", "Version": "2.4.5"}"#);
        let value = resolve_version(&params("devtools"), &fetcher).unwrap();
        assert_eq!(value, "2.4.5");
    }

    #[test]
    fn requires_package_param() {
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
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"License": "MIT"}"#);
        assert!(resolve_version(&params("devtools"), &fetcher).is_err());
    }
}
