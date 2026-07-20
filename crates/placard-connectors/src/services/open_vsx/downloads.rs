use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let namespace = params
        .get("namespace")
        .ok_or("open-vsx-downloads requires a data-namespace attribute")?;
    let namespace = validate_path_param("namespace", namespace)?;
    let extension = params
        .get("extension")
        .ok_or("open-vsx-downloads requires a data-extension attribute")?;
    let extension = validate_path_param("extension", extension)?;

    let mut url = format!("https://open-vsx.org/api/{namespace}/{extension}");
    if let Some(version) = params.get("version") {
        let version = validate_path_param("version", version)?;
        url.push('/');
        url.push_str(version);
    }

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "open-vsx response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let downloads = value
        .get("downloadCount")
        .ok_or("open-vsx response missing downloadCount")?;
    downloads
        .as_text()
        .ok_or_else(|| "downloadCount was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: &'static str,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(namespace: &str, extension: &str) -> HashMap<String, String> {
        HashMap::from([
            ("namespace".to_string(), namespace.to_string()),
            ("extension".to_string(), extension.to_string()),
        ])
    }

    #[test]
    fn extracts_download_count() {
        let fetcher = FakeFetcher {
            expected_url: "https://open-vsx.org/api/redhat/java",
            body: r#"{"version": "1.0.0", "timestamp": "2024-01-01T00:00:00Z", "downloadCount": 12345}"#,
        };
        let value = resolve_downloads(&params("redhat", "java"), &fetcher).unwrap();
        assert_eq!(value, "12345");
    }

    #[test]
    fn appends_version_when_provided() {
        let mut p = params("redhat", "java");
        p.insert("version".to_string(), "0.69.0".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://open-vsx.org/api/redhat/java/0.69.0",
            body: r#"{"version": "0.69.0", "timestamp": "2024-01-01T00:00:00Z", "downloadCount": 500}"#,
        };
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "500");
    }

    #[test]
    fn requires_namespace_and_extension_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("redhat", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc", "java"), &Unused).is_err());
    }

    #[test]
    fn errors_when_download_count_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://open-vsx.org/api/redhat/java",
            body: r#"{"version": "1.0.0", "timestamp": "2024-01-01T00:00:00Z"}"#,
        };
        assert!(resolve_downloads(&params("redhat", "java"), &fetcher).is_err());
    }
}
