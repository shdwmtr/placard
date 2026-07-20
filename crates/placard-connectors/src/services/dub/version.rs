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
        .ok_or("dub-version requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://code.dlang.org/api/packages/{package}/latest");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "dub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .as_text()
        .ok_or_else(|| "dub response was not a plain version string".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://code.dlang.org/api/packages/vibe-d/latest");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_the_plain_version_string() {
        let fetcher = FakeFetcher(r#""0.9.5""#);
        let value = resolve_version(&params("vibe-d"), &fetcher).unwrap();
        assert_eq!(value, "0.9.5");
    }

    #[test]
    fn requires_package_param() {
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
    fn errors_when_the_response_is_not_a_plain_string() {
        let fetcher = FakeFetcher(r#"{"version": "0.9.5"}"#);
        assert!(resolve_version(&params("vibe-d"), &fetcher).is_err());
    }
}
