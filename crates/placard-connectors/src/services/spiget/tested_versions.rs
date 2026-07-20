use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_tested_versions(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let resource_id = params
        .get("resource-id")
        .ok_or("spiget-tested-versions requires a data-resource-id attribute")?;
    let resource_id = validate_path_param("resource-id", resource_id)?;

    let url = format!("https://api.spiget.org/v2/resources/{resource_id}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "spiget response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let Some(Value::Array(versions)) = value.get("testedVersions") else {
        return Err("spiget response missing testedVersions".to_string());
    };
    if versions.is_empty() {
        return Err("spiget response had an empty testedVersions array".to_string());
    }
    let earliest = versions
        .first()
        .and_then(Value::as_text)
        .ok_or("testedVersions entry was not a plain value")?;
    let latest = versions
        .last()
        .and_then(Value::as_text)
        .ok_or("testedVersions entry was not a plain value")?;

    Ok(if earliest == latest {
        earliest
    } else {
        format!("{earliest}-{latest}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.spiget.org/v2/resources/9089");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(resource_id: &str) -> HashMap<String, String> {
        HashMap::from([("resource-id".to_string(), resource_id.to_string())])
    }

    #[test]
    fn extracts_a_range_from_multiple_tested_versions() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 1, "file": {"type": "jar", "size": 1, "sizeUnit": "MB"}, "rating": {"count": 1, "average": 5}, "testedVersions": ["1.16", "1.17", "1.18"]}"#,
        );
        let value = resolve_tested_versions(&params("9089"), &fetcher).unwrap();
        assert_eq!(value, "1.16-1.18");
    }

    #[test]
    fn returns_a_single_version_when_only_one_is_tested() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 1, "file": {"type": "jar", "size": 1, "sizeUnit": "MB"}, "rating": {"count": 1, "average": 5}, "testedVersions": ["1.18"]}"#,
        );
        let value = resolve_tested_versions(&params("9089"), &fetcher).unwrap();
        assert_eq!(value, "1.18");
    }

    #[test]
    fn requires_resource_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_tested_versions(&HashMap::new(), &Unused).is_err());
        assert!(resolve_tested_versions(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_tested_versions(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_tested_versions_field_is_missing() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 1, "file": {"type": "jar", "size": 1, "sizeUnit": "MB"}}"#,
        );
        assert!(resolve_tested_versions(&params("9089"), &fetcher).is_err());
    }
}
