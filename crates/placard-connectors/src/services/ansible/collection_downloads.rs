use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_collection_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let namespace = params
        .get("namespace")
        .ok_or("ansible-collection-downloads requires a data-namespace attribute")?;
    let name = params
        .get("name")
        .ok_or("ansible-collection-downloads requires a data-name attribute")?;
    let namespace = validate_path_param("namespace", namespace)?;
    let name = validate_path_param("name", name)?;

    let url = format!(
        "https://galaxy.ansible.com/api/v3/plugin/ansible/content/published/collections/index/{namespace}/{name}/"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "ansible galaxy response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("download_count")
        .ok_or("ansible galaxy response missing download_count")?;
    count
        .as_text()
        .ok_or_else(|| "download_count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://galaxy.ansible.com/api/v3/plugin/ansible/content/published/collections/index/community/general/"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(namespace: &str, name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("namespace".to_string(), namespace.to_string()),
            ("name".to_string(), name.to_string()),
        ])
    }

    #[test]
    fn extracts_download_count() {
        let fetcher = FakeFetcher(r#"{"download_count": 1234567}"#);
        let value =
            resolve_collection_downloads(&params("community", "general"), &fetcher).unwrap();
        assert_eq!(value, "1234567");
    }

    #[test]
    fn requires_namespace_and_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_collection_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_collection_downloads(&params("community", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_collection_downloads(&params("../etc", "general"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"other": 1}"#);
        assert!(resolve_collection_downloads(&params("community", "general"), &fetcher).is_err());
    }
}
