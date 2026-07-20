use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

/// Mirrors shields' `crates/d`, `crates/dv`, and `crates/dr` badges, selected
/// via `data-variant` (defaults to `d`, the crate's all-time total):
/// - `d`: total downloads for the crate.
/// - `dv`: downloads for a specific version (the crate's latest if
///   `data-version` is omitted).
/// - `dr`: recent downloads for the crate (not available per-version).
pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let crate_name = params
        .get("crate")
        .ok_or("crates-downloads requires a data-crate attribute")?;
    let crate_name = validate_path_param("crate", crate_name)?;
    let variant = match params.get("variant").map(String::as_str) {
        Some(v @ ("d" | "dv" | "dr")) => v,
        Some(_) => {
            return Err("crates-downloads data-variant must be one of 'd', 'dv', 'dr'".to_string());
        }
        None => "d",
    };
    let version = match params.get("version") {
        Some(v) => Some(validate_path_param("version", v)?),
        None => None,
    };

    if variant == "dr" && version.is_some() {
        return Err(
            "crates-downloads recent downloads are not supported for specific versions".to_string(),
        );
    }

    let url = match version {
        Some(version) => format!("https://crates.io/api/v1/crates/{crate_name}/{version}"),
        None => format!("https://crates.io/api/v1/crates/{crate_name}?include=versions,downloads"),
    };
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "crates.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let downloads = match variant {
        "dv" => {
            let version_obj = version_object(&value)?;
            version_obj
                .get("downloads")
                .ok_or("crates.io response missing downloads")?
        }
        "dr" => {
            return Ok(value
                .get("crate.recent_downloads")
                .and_then(|v| v.as_text())
                .unwrap_or_else(|| "0".to_string()));
        }
        _ => {
            if let Some(crate_obj) = value.get("crate") {
                crate_obj
                    .get("downloads")
                    .ok_or("crates.io response missing downloads")?
            } else {
                value
                    .get("version")
                    .and_then(|v| v.get("downloads"))
                    .ok_or("crates.io response missing downloads")?
            }
        }
    };

    downloads
        .as_text()
        .ok_or_else(|| "downloads was not a plain value".to_string())
}

/// Resolves the version record a crates.io crate/version response describes:
/// the matching entry of `versions` for the crate's latest stable (or, failing
/// that, latest) version, or the `version` object directly when a specific
/// version was requested.
fn version_object(value: &Value) -> Result<&Value, String> {
    if let Some(crate_obj) = value.get("crate") {
        let latest = crate_obj
            .get("max_stable_version")
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .or_else(|| crate_obj.get("max_version").and_then(|v| v.as_text()))
            .ok_or("crates.io response missing max_version")?;

        let versions = match value.get("versions") {
            Some(Value::Array(items)) => items,
            _ => return Err("crates.io response missing versions array".to_string()),
        };
        versions
            .iter()
            .find(|item| {
                item.get("num").and_then(|v| v.as_text()).as_deref() == Some(latest.as_str())
            })
            .ok_or_else(|| "version not found in crates.io response".to_string())
    } else {
        value
            .get("version")
            .ok_or_else(|| "crates.io response missing version".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    fn params(
        crate_name: &str,
        variant: Option<&str>,
        version: Option<&str>,
    ) -> HashMap<String, String> {
        let mut map = HashMap::from([("crate".to_string(), crate_name.to_string())]);
        if let Some(v) = variant {
            map.insert("variant".to_string(), v.to_string());
        }
        if let Some(v) = version {
            map.insert("version".to_string(), v.to_string());
        }
        map
    }

    #[test]
    fn extracts_total_downloads_by_default() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/rustc-serialize?include=versions,downloads",
            r#"{"crate": {"downloads": 91234567, "max_version": "0.3.24"}, "versions": []}"#,
        );
        let value = resolve_downloads(&params("rustc-serialize", None, None), &fetcher).unwrap();
        assert_eq!(value, "91234567");
    }

    #[test]
    fn extracts_recent_downloads_for_dr_variant() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/rustc-serialize?include=versions,downloads",
            r#"{"crate": {"downloads": 1, "recent_downloads": 4200, "max_version": "0.3.24"}, "versions": []}"#,
        );
        let value =
            resolve_downloads(&params("rustc-serialize", Some("dr"), None), &fetcher).unwrap();
        assert_eq!(value, "4200");
    }

    #[test]
    fn extracts_downloads_for_a_specific_version_with_dv_variant() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/rustc-serialize/0.3.24",
            r#"{"version": {"num": "0.3.24", "downloads": 555}}"#,
        );
        let value = resolve_downloads(
            &params("rustc-serialize", Some("dv"), Some("0.3.24")),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "555");
    }

    #[test]
    fn extracts_downloads_for_the_latest_version_with_dv_variant_and_no_version() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/rustc-serialize?include=versions,downloads",
            r#"{"crate": {"max_stable_version": "0.3.24", "max_version": "0.3.24"}, "versions": [{"num": "0.3.24", "downloads": 555}]}"#,
        );
        let value =
            resolve_downloads(&params("rustc-serialize", Some("dv"), None), &fetcher).unwrap();
        assert_eq!(value, "555");
    }

    #[test]
    fn rejects_recent_downloads_with_a_specific_version() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch dr with a version")
            }
        }
        assert!(
            resolve_downloads(
                &params("rustc-serialize", Some("dr"), Some("0.3.24")),
                &Unused
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_unknown_variant() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid variant")
            }
        }
        assert!(
            resolve_downloads(&params("rustc-serialize", Some("bogus"), None), &Unused).is_err()
        );
    }

    #[test]
    fn requires_crate_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid crate param")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("", None, None), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc", None, None), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_downloads_field_is_missing() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/rustc-serialize?include=versions,downloads",
            r#"{"crate": {"max_version": "0.3.24"}, "versions": []}"#,
        );
        assert!(resolve_downloads(&params("rustc-serialize", None, None), &fetcher).is_err());
    }
}
