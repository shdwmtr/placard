use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_latest_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let resource_id = params
        .get("resource-id")
        .ok_or("spiget-latest-version requires a data-resource-id attribute")?;
    let resource_id = validate_path_param("resource-id", resource_id)?;

    let url = format!("https://api.spiget.org/v2/resources/{resource_id}/versions/latest");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "spiget response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let name = value.get("name").ok_or("spiget response missing name")?;
    name.as_text()
        .ok_or_else(|| "name was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.spiget.org/v2/resources/9089/versions/latest"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(resource_id: &str) -> HashMap<String, String> {
        HashMap::from([("resource-id".to_string(), resource_id.to_string())])
    }

    #[test]
    fn extracts_the_version_name_from_a_spiget_shaped_response() {
        let fetcher = FakeFetcher(r#"{"downloads": 100, "name": "2.5.1"}"#);
        let value = resolve_latest_version(&params("9089"), &fetcher).unwrap();
        assert_eq!(value, "2.5.1");
    }

    #[test]
    fn requires_resource_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_latest_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_latest_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_latest_version(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_name_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"downloads": 100}"#);
        assert!(resolve_latest_version(&params("9089"), &fetcher).is_err());
    }
}
