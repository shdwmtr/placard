use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let namespace = params
        .get("namespace")
        .ok_or("open-vsx-version requires a data-namespace attribute")?;
    let namespace = validate_path_param("namespace", namespace)?;
    let extension = params
        .get("extension")
        .ok_or("open-vsx-version requires a data-extension attribute")?;
    let extension = validate_path_param("extension", extension)?;

    let url = format!("https://open-vsx.org/api/{namespace}/{extension}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "open-vsx response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("open-vsx response missing version")?;
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
            assert_eq!(url, "https://open-vsx.org/api/redhat/java");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(namespace: &str, extension: &str) -> HashMap<String, String> {
        HashMap::from([
            ("namespace".to_string(), namespace.to_string()),
            ("extension".to_string(), extension.to_string()),
        ])
    }

    #[test]
    fn extracts_the_version() {
        let fetcher = FakeFetcher(r#"{"version": "1.14.0", "timestamp": "2024-03-15T10:30:00Z"}"#);
        let value = resolve_version(&params("redhat", "java"), &fetcher).unwrap();
        assert_eq!(value, "1.14.0");
    }

    #[test]
    fn requires_namespace_and_extension_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("redhat", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc", "java"), &Unused).is_err());
    }

    #[test]
    fn errors_when_version_is_missing() {
        let fetcher = FakeFetcher(r#"{"timestamp": "2024-03-15T10:30:00Z"}"#);
        assert!(resolve_version(&params("redhat", "java"), &fetcher).is_err());
    }
}
