use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_plugin_installs(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let plugin = params
        .get("plugin")
        .ok_or("jenkins-plugin-installs requires a data-plugin attribute")?;
    let plugin = validate_path_param("plugin", plugin)?;

    let url = format!("https://stats.jenkins.io/plugin-installation-trend/{plugin}.stats.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "jenkins response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let installations = value
        .get("installations")
        .ok_or("jenkins response missing installations")?;
    let Value::Object(fields) = installations else {
        return Err("installations was not an object".to_string());
    };
    let latest = fields
        .iter()
        .max_by(|a, b| a.0.cmp(&b.0))
        .ok_or_else(|| "installations was empty".to_string())?;

    latest
        .1
        .as_text()
        .ok_or_else(|| "latest install count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://stats.jenkins.io/plugin-installation-trend/view-job-filters.stats.json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(plugin: &str) -> HashMap<String, String> {
        HashMap::from([("plugin".to_string(), plugin.to_string())])
    }

    #[test]
    fn extracts_the_most_recent_install_count() {
        let fetcher =
            FakeFetcher(r#"{"installations": {"1700000000000": 4200, "1700086400000": 4310}}"#);
        let value = resolve_plugin_installs(&params("view-job-filters"), &fetcher).unwrap();
        assert_eq!(value, "4310");
    }

    #[test]
    fn requires_plugin_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a plugin")
            }
        }
        assert!(resolve_plugin_installs(&HashMap::new(), &Unused).is_err());
        assert!(resolve_plugin_installs(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_plugin_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid plugin")
            }
        }
        assert!(resolve_plugin_installs(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_installations_is_missing() {
        struct FakeFetcherMissing;
        impl Fetcher for FakeFetcherMissing {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Ok(br#"{}"#.to_vec())
            }
        }
        assert!(resolve_plugin_installs(&params("view-job-filters"), &FakeFetcherMissing).is_err());
    }

    #[test]
    fn errors_when_installations_is_empty() {
        struct EmptyFetcher;
        impl Fetcher for EmptyFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Ok(br#"{"installations": {}}"#.to_vec())
            }
        }
        assert!(resolve_plugin_installs(&params("view-job-filters"), &EmptyFetcher).is_err());
    }
}
