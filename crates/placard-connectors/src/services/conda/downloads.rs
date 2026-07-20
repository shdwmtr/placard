use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let channel = params
        .get("channel")
        .ok_or("conda-downloads requires a data-channel attribute")?;
    let package = params
        .get("package")
        .ok_or("conda-downloads requires a data-package attribute")?;
    let channel = validate_path_param("channel", channel)?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://api.anaconda.org/package/{channel}/{package}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "conda response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let files = match value.get("files") {
        Some(Value::Array(items)) => items,
        _ => return Err("conda response missing files array".to_string()),
    };

    let mut total = 0i64;
    for file in files {
        let count = file
            .get("ndownloads")
            .and_then(Value::as_text)
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or("conda file missing ndownloads")?;
        total += count;
    }
    Ok(total.to_string())
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
    fn sums_ndownloads_across_files() {
        let fetcher = FakeFetcher(
            r#"{"latest_version": "3.11", "conda_platforms": ["linux-64"], "files": [{"ndownloads": 100}, {"ndownloads": 42}]}"#,
        );
        let value = resolve_downloads(&params("conda-forge", "python"), &fetcher).unwrap();
        assert_eq!(value, "142");
    }

    #[test]
    fn requires_channel_and_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("conda-forge", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc", "python"), &Unused).is_err());
    }

    #[test]
    fn errors_when_files_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"latest_version": "3.11", "conda_platforms": []}"#);
        assert!(resolve_downloads(&params("conda-forge", "python"), &fetcher).is_err());
    }
}
