use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let plugin_id = params
        .get("plugin-id")
        .ok_or("jetbrains-version requires a data-plugin-id attribute")?;
    let plugin_id = validate_path_param("plugin-id", plugin_id)?;

    let url = format!("https://plugins.jetbrains.com/api/plugins/{plugin_id}/updates");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "jetbrains response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let updates = match &value {
        Value::Array(items) => items,
        _ => return Err("jetbrains response was not an array".to_string()),
    };
    let latest = updates.first().ok_or("jetbrains response had no updates")?;
    latest
        .get("version")
        .and_then(Value::as_text)
        .ok_or_else(|| "jetbrains update missing version".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://plugins.jetbrains.com/api/plugins/9630/updates"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(plugin_id: &str) -> HashMap<String, String> {
        HashMap::from([("plugin-id".to_string(), plugin_id.to_string())])
    }

    #[test]
    fn extracts_version_from_the_latest_update() {
        let fetcher =
            FakeFetcher(r#"[{"id": 1, "version": "1.4.2"}, {"id": 2, "version": "1.4.1"}]"#);
        let value = resolve_version(&params("9630"), &fetcher).unwrap();
        assert_eq!(value, "1.4.2");
    }

    #[test]
    fn requires_plugin_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_updates() {
        let fetcher = FakeFetcher("[]");
        assert!(resolve_version(&params("9630"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_version_field_is_missing() {
        let fetcher = FakeFetcher(r#"[{"id": 1}]"#);
        assert!(resolve_version(&params("9630"), &fetcher).is_err());
    }
}
