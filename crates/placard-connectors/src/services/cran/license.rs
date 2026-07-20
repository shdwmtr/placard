use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("cran-license requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("http://crandb.r-pkg.org/{package}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "cran response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let license = value
        .get("License")
        .ok_or("cran response missing License")?;
    license
        .as_text()
        .ok_or_else(|| "License was not a plain value".to_string())
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
    fn extracts_license_from_a_cran_shaped_response() {
        let fetcher = FakeFetcher(r#"{"License": "MIT + file LICENSE", "Version": "2.4.5"}"#);
        let value = resolve_license(&params("devtools"), &fetcher).unwrap();
        assert_eq!(value, "MIT + file LICENSE");
    }

    #[test]
    fn requires_package_param() {
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
        assert!(resolve_license(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"Version": "2.4.5"}"#);
        assert!(resolve_license(&params("devtools"), &fetcher).is_err());
    }
}
