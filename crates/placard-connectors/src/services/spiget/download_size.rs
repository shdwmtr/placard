use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_download_size(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let resource_id = params
        .get("resource-id")
        .ok_or("spiget-download-size requires a data-resource-id attribute")?;
    let resource_id = validate_path_param("resource-id", resource_id)?;

    let url = format!("https://api.spiget.org/v2/resources/{resource_id}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "spiget response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let file_type = value
        .get("file.type")
        .and_then(|v| v.as_text())
        .ok_or("spiget response missing file.type")?;
    if file_type == "external" {
        return Ok("resource hosted externally".to_string());
    }

    let size = value
        .get("file.size")
        .and_then(|v| v.as_text())
        .ok_or("spiget response missing file.size")?;
    let unit = value
        .get("file.sizeUnit")
        .and_then(|v| v.as_text())
        .ok_or("spiget response missing file.sizeUnit")?;

    Ok(format!("{size} {unit}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.spiget.org/v2/resources/15904");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(resource_id: &str) -> HashMap<String, String> {
        HashMap::from([("resource-id".to_string(), resource_id.to_string())])
    }

    #[test]
    fn extracts_size_and_unit_from_a_spiget_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 10, "file": {"type": "jar", "size": 512, "sizeUnit": "KB"}, "rating": {"count": 1, "average": 5}}"#,
        );
        let value = resolve_download_size(&params("15904"), &fetcher).unwrap();
        assert_eq!(value, "512 KB");
    }

    #[test]
    fn reports_externally_hosted_resources() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 10, "file": {"type": "external", "size": 0, "sizeUnit": ""}, "rating": {"count": 1, "average": 5}}"#,
        );
        let value = resolve_download_size(&params("15904"), &fetcher).unwrap();
        assert_eq!(value, "resource hosted externally");
    }

    #[test]
    fn requires_resource_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_download_size(&HashMap::new(), &Unused).is_err());
        assert!(resolve_download_size(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_download_size(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_file_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"downloads": 10, "rating": {"count": 1, "average": 5}}"#);
        assert!(resolve_download_size(&params("15904"), &fetcher).is_err());
    }
}
