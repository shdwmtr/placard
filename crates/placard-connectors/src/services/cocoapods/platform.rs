use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_platform(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let spec = params
        .get("spec")
        .ok_or("cocoapods-platform requires a data-spec attribute")?;
    let spec = validate_path_param("spec", spec)?;

    let url = format!("https://trunk.cocoapods.org/api/v1/pods/{spec}/specs/latest");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "cocoapods response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let keys: Vec<String> = match value.get("platforms") {
        Some(Value::Object(fields)) => fields.iter().map(|(k, _)| k.clone()).collect(),
        None => vec!["ios".to_string(), "osx".to_string()],
        Some(_) => return Err("cocoapods response platforms field was not an object".to_string()),
    };

    Ok(keys.join(" | "))
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
    fn extracts_the_platform_keys_joined_with_a_pipe() {
        let fetcher =
            FakeFetcher(r#"{"version": "4.0.1", "platforms": {"ios": "6.0", "osx": "10.8"}}"#);
        let value = resolve_platform(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "ios | osx");
    }

    #[test]
    fn defaults_to_ios_and_osx_when_platforms_is_missing() {
        let fetcher = FakeFetcher(r#"{"version": "4.0.1"}"#);
        let value = resolve_platform(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "ios | osx");
    }

    #[test]
    fn requires_spec_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid spec")
            }
        }
        assert!(resolve_platform(&HashMap::new(), &Unused).is_err());
        assert!(resolve_platform(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_platform(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_platforms_is_not_an_object() {
        let fetcher = FakeFetcher(r#"{"version": "4.0.1", "platforms": "ios"}"#);
        assert!(resolve_platform(&params("AFNetworking"), &fetcher).is_err());
    }
}
