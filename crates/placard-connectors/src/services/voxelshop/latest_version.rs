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
        .ok_or("voxelshop-latest-version requires a data-resource-id attribute")?;
    let resource_id = validate_path_param("resource-id", resource_id)?;

    let url = format!("https://api.voxel.shop/v1/getResourceInfo/?resource_id={resource_id}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "voxelshop response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("response.resource.updates.latest.version")
        .ok_or("voxelshop response missing response.resource.updates.latest.version")?;
    version
        .as_text()
        .ok_or_else(|| "response.resource.updates.latest.version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.voxel.shop/v1/getResourceInfo/?resource_id=323"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(resource_id: &str) -> HashMap<String, String> {
        HashMap::from([("resource-id".to_string(), resource_id.to_string())])
    }

    #[test]
    fn extracts_latest_version_from_a_voxelshop_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"response":{"resource":{"downloads":48213,"reviews":{"count":10,"stars":4.5},"updates":{"latest":{"version":"2.4.1"}}}}}"#,
        );
        let value = resolve_latest_version(&params("323"), &fetcher).unwrap();
        assert_eq!(value, "2.4.1");
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
    fn errors_when_resource_is_not_found() {
        let fetcher =
            FakeFetcher(r#"{"response":{"success":false,"errors":{"resource":"not found"}}}"#);
        assert!(resolve_latest_version(&params("323"), &fetcher).is_err());
    }
}
