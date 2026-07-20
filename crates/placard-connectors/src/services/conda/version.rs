use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let channel = params
        .get("channel")
        .ok_or("conda-version requires a data-channel attribute")?;
    let package = params
        .get("package")
        .ok_or("conda-version requires a data-package attribute")?;
    let channel = validate_path_param("channel", channel)?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://api.anaconda.org/package/{channel}/{package}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "conda response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("latest_version")
        .ok_or("conda response missing latest_version")?;
    version
        .as_text()
        .ok_or_else(|| "latest_version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.anaconda.org/package/conda-forge/python");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(channel: &str, package: &str) -> HashMap<String, String> {
        HashMap::from([
            ("channel".to_string(), channel.to_string()),
            ("package".to_string(), package.to_string()),
        ])
    }

    #[test]
    fn extracts_the_latest_version_field() {
        let fetcher = FakeFetcher(r#"{"latest_version": "3.11.0"}"#);
        let value = resolve_version(&params("conda-forge", "python"), &fetcher).unwrap();
        assert_eq!(value, "3.11.0");
    }

    #[test]
    fn requires_channel_and_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("conda-forge", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc", "python"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"conda_platforms": ["linux-64"]}"#);
        assert!(resolve_version(&params("conda-forge", "python"), &fetcher).is_err());
    }
}
