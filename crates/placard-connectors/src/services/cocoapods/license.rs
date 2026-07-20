use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let spec = params
        .get("spec")
        .ok_or("cocoapods-license requires a data-spec attribute")?;
    let spec = validate_path_param("spec", spec)?;

    let url = format!("https://trunk.cocoapods.org/api/v1/pods/{spec}/specs/latest");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "cocoapods response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let license_text = match value.get("license") {
        Some(Value::Object(_)) => value.get("license.type").and_then(Value::as_text),
        Some(other) => other.as_text(),
        None => None,
    };

    Ok(license_text.unwrap_or_else(|| "not specified".to_string()))
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
    fn extracts_a_plain_string_license() {
        let fetcher = FakeFetcher(r#"{"version": "4.0.1", "license": "MIT"}"#);
        let value = resolve_license(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn extracts_the_type_field_from_an_object_license() {
        let fetcher =
            FakeFetcher(r#"{"version": "4.0.1", "license": {"type": "BSD", "text": "..."}}"#);
        let value = resolve_license(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "BSD");
    }

    #[test]
    fn falls_back_to_not_specified_when_license_is_missing() {
        let fetcher = FakeFetcher(r#"{"version": "4.0.1"}"#);
        let value = resolve_license(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "not specified");
    }

    #[test]
    fn requires_spec_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid spec")
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
}
