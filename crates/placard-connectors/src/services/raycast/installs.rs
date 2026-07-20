use crate::Fetcher;
use crate::json;
use crate::services::validate_path_param;
use std::collections::HashMap;

pub(crate) fn resolve_installs(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("installs requires a data-user attribute")?;
    let extension = params
        .get("extension")
        .ok_or("installs requires a data-extension attribute")?;
    let user = validate_path_param("user", user)?;
    let extension = validate_path_param("extension", extension)?;

    let url = format!("https://www.raycast.com/api/v1/extensions/{user}/{extension}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "raycast response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let downloads = value
        .get("download_count")
        .ok_or("raycast response missing download_count")?;
    downloads
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
                "https://www.raycast.com/api/v1/extensions/Fatpandac/bilibili"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, extension: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("extension".to_string(), extension.to_string()),
        ])
    }

    #[test]
    fn extracts_the_download_count() {
        let fetcher = FakeFetcher(r#"{"download_count": 4213}"#);
        let value = resolve_installs(&params("Fatpandac", "bilibili"), &fetcher).unwrap();
        assert_eq!(value, "4213");
    }

    #[test]
    fn requires_user_and_extension_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_installs(&HashMap::new(), &Unused).is_err());
        assert!(resolve_installs(&params("Fatpandac", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_installs(&params("../etc", "bilibili"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "bilibili"}"#);
        assert!(resolve_installs(&params("Fatpandac", "bilibili"), &fetcher).is_err());
    }
}
