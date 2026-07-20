use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("flathub-version requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://flathub.org/api/v2/appstream/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "flathub response was not valid UTF-8".to_string())?;
    let root = json::parse(&text)?;
    let releases = root
        .get("releases")
        .ok_or("flathub response missing releases")?;
    let Value::Array(items) = releases else {
        return Err("flathub response's releases field was not an array".to_string());
    };

    let mut latest: Option<(i64, &Value)> = None;
    for item in items {
        let timestamp = item
            .get("timestamp")
            .and_then(Value::as_text)
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or("flathub release entry missing a numeric timestamp")?;
        if latest.map(|(best, _)| timestamp > best).unwrap_or(true) {
            latest = Some((timestamp, item));
        }
    }

    let latest = latest.ok_or("flathub response had no releases")?.1;
    let version = latest
        .get("version")
        .ok_or("flathub release entry missing version")?;
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
                "https://flathub.org/api/v2/appstream/org.mozilla.firefox"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn picks_the_release_with_the_latest_timestamp() {
        let fetcher = FakeFetcher(
            r#"{"releases": [
                {"timestamp": "100", "version": "1.0.0"},
                {"timestamp": "300", "version": "3.0.0"},
                {"timestamp": "200", "version": "2.0.0"}
            ]}"#,
        );
        let value = resolve_version(&params("org.mozilla.firefox"), &fetcher).unwrap();
        assert_eq!(value, "3.0.0");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
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
    fn errors_when_releases_is_missing_or_empty() {
        let fetcher = FakeFetcher(r#"{"releases": []}"#);
        assert!(resolve_version(&params("org.mozilla.firefox"), &fetcher).is_err());

        let fetcher = FakeFetcher(r#"{"other": 1}"#);
        assert!(resolve_version(&params("org.mozilla.firefox"), &fetcher).is_err());
    }
}
