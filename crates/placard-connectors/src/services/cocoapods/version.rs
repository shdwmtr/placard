use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let spec = params
        .get("spec")
        .ok_or("cocoapods-version requires a data-spec attribute")?;
    let spec = validate_path_param("spec", spec)?;

    let url = format!("https://trunk.cocoapods.org/api/v1/pods/{spec}/specs/latest");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "cocoapods response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("cocoapods response missing version")?;
    version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://trunk.cocoapods.org/api/v1/pods/AFNetworking/specs/latest"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(spec: &str) -> HashMap<String, String> {
        HashMap::from([("spec".to_string(), spec.to_string())])
    }

    #[test]
    fn extracts_the_version_field() {
        let fetcher = FakeFetcher(r#"{"version": "4.0.1", "license": "MIT"}"#);
        let value = resolve_version(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "4.0.1");
    }

    #[test]
    fn requires_spec_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid spec")
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
    fn errors_when_the_version_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"license": "MIT"}"#);
        assert!(resolve_version(&params("AFNetworking"), &fetcher).is_err());
    }
}
