use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_docs(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let spec = params
        .get("spec")
        .ok_or("cocoapods-docs requires a data-spec attribute")?;
    let spec = validate_path_param("spec", spec)?;

    let url = format!("https://metrics.cocoapods.org/api/v1/pods/{spec}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "cocoapods response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let cocoadocs = value
        .get("cocoadocs")
        .ok_or("cocoapods response missing cocoadocs")?;
    let doc_percent = cocoadocs
        .get("doc_percent")
        .ok_or("cocoapods response missing doc_percent")?;
    Ok(doc_percent.as_text().unwrap_or_else(|| "0".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://metrics.cocoapods.org/api/v1/pods/AFNetworking"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(spec: &str) -> HashMap<String, String> {
        HashMap::from([("spec".to_string(), spec.to_string())])
    }

    #[test]
    fn extracts_the_doc_percent_field() {
        let fetcher = FakeFetcher(r#"{"cocoadocs": {"doc_percent": 87}}"#);
        let value = resolve_docs(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "87");
    }

    #[test]
    fn defaults_to_zero_when_doc_percent_is_null() {
        let fetcher = FakeFetcher(r#"{"cocoadocs": {"doc_percent": null}}"#);
        let value = resolve_docs(&params("AFNetworking"), &fetcher).unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn requires_spec_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid spec")
            }
        }
        assert!(resolve_docs(&HashMap::new(), &Unused).is_err());
        assert!(resolve_docs(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_docs(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_cocoadocs_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"version": "1.0"}"#);
        assert!(resolve_docs(&params("AFNetworking"), &fetcher).is_err());
    }
}
