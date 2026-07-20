use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let plugin_id = params
        .get("plugin-id")
        .ok_or("jetbrains-downloads requires a data-plugin-id attribute")?;
    let plugin_id = validate_path_param("plugin-id", plugin_id)?;

    let url = format!("https://plugins.jetbrains.com/api/plugins/{plugin_id}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "jetbrains response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("downloads")
        .and_then(Value::as_text)
        .ok_or_else(|| "jetbrains response missing downloads".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://plugins.jetbrains.com/api/plugins/1347");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(plugin_id: &str) -> HashMap<String, String> {
        HashMap::from([("plugin-id".to_string(), plugin_id.to_string())])
    }

    #[test]
    fn extracts_downloads_from_a_jetbrains_shaped_response() {
        let fetcher = FakeFetcher(r#"{"id": 1347, "name": ".ignore", "downloads": 5123456}"#);
        let value = resolve_downloads(&params("1347"), &fetcher).unwrap();
        assert_eq!(value, "5123456");
    }

    #[test]
    fn requires_plugin_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"id": 1347}"#);
        assert!(resolve_downloads(&params("1347"), &fetcher).is_err());
    }
}
