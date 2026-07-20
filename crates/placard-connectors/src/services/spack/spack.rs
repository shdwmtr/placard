use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_spack(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("spack requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://packages.spack.io/data/packages/{package_name}.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "spack response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("latest_version")
        .ok_or("spack response missing latest_version")?
        .as_text()
        .ok_or_else(|| "latest_version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://packages.spack.io/data/packages/adios2.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_the_latest_version() {
        let fetcher = FakeFetcher(r#"{"latest_version": "2.8.3"}"#);
        let value = resolve_spack(&params("adios2"), &fetcher).unwrap();
        assert_eq!(value, "2.8.3");
    }

    #[test]
    fn requires_a_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_spack(&HashMap::new(), &Unused).is_err());
        assert!(resolve_spack(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_spack(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"other": 1}"#);
        assert!(resolve_spack(&params("adios2"), &fetcher).is_err());
    }
}
