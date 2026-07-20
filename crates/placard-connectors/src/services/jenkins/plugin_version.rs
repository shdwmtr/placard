use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_plugin_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let plugin = params
        .get("plugin")
        .ok_or("jenkins-plugin-version requires a data-plugin attribute")?;
    if plugin.is_empty() {
        return Err("'plugin' parameter must not be empty".to_string());
    }

    let url = "https://updates.jenkins-ci.org/current/update-center.actual.json";
    let bytes = fetcher.fetch(url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "jenkins response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let plugins = value
        .get("plugins")
        .ok_or("jenkins response missing plugins")?;
    let Value::Object(fields) = plugins else {
        return Err("plugins was not an object".to_string());
    };
    let entry = fields
        .iter()
        .find(|(key, _)| key == plugin)
        .map(|(_, entry)| entry)
        .ok_or("plugin not found")?;
    let version = entry
        .get("version")
        .ok_or("jenkins plugin entry missing version")?;
    version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://updates.jenkins-ci.org/current/update-center.actual.json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(plugin: &str) -> HashMap<String, String> {
        HashMap::from([("plugin".to_string(), plugin.to_string())])
    }

    #[test]
    fn extracts_the_plugin_version() {
        let fetcher = FakeFetcher(r#"{"plugins": {"blueocean": {"version": "1.1.6"}}}"#);
        let value = resolve_plugin_version(&params("blueocean"), &fetcher).unwrap();
        assert_eq!(value, "1.1.6");
    }

    #[test]
    fn requires_plugin_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a plugin")
            }
        }
        assert!(resolve_plugin_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_plugin_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_plugin_is_not_present() {
        let fetcher = FakeFetcher(r#"{"plugins": {"blueocean": {"version": "1.1.6"}}}"#);
        assert!(resolve_plugin_version(&params("inexistent-artifact-id"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_plugins_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_plugin_version(&params("blueocean"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_version_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"plugins": {"blueocean": {}}}"#);
        assert!(resolve_plugin_version(&params("blueocean"), &fetcher).is_err());
    }
}
