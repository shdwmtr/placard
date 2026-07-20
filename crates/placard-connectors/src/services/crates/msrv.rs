use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_msrv(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let crate_name = params
        .get("crate")
        .ok_or("crates-msrv requires a data-crate attribute")?;
    let crate_name = validate_path_param("crate", crate_name)?;
    let version = match params.get("version") {
        Some(v) => Some(validate_path_param("version", v)?),
        None => None,
    };

    let url = match version {
        Some(version) => format!("https://crates.io/api/v1/crates/{crate_name}/{version}"),
        None => format!("https://crates.io/api/v1/crates/{crate_name}?include=versions,downloads"),
    };
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "crates.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version_obj = version_object(&value)?;

    version_obj
        .get("rust_version")
        .and_then(|v| v.as_text())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "crate has no msrv on crates.io".to_string())
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

    fn params(crate_name: &str, version: Option<&str>) -> HashMap<String, String> {
        let mut map = HashMap::from([("crate".to_string(), crate_name.to_string())]);
        if let Some(v) = version {
            map.insert("version".to_string(), v.to_string());
        }
        map
    }

    #[test]
    fn extracts_msrv_for_the_latest_version_when_no_version_given() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/serde?include=versions,downloads",
            r#"{"crate": {"max_stable_version": "1.0.194", "max_version": "1.0.194"}, "versions": [{"num": "1.0.194", "rust_version": "1.56"}]}"#,
        );
        let value = resolve_msrv(&params("serde", None), &fetcher).unwrap();
        assert_eq!(value, "1.56");
    }

    #[test]
    fn extracts_msrv_for_an_explicit_version() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/serde/1.0.194",
            r#"{"version": {"num": "1.0.194", "rust_version": "1.56"}}"#,
        );
        let value = resolve_msrv(&params("serde", Some("1.0.194")), &fetcher).unwrap();
        assert_eq!(value, "1.56");
    }

    #[test]
    fn requires_crate_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid crate param")
            }
        }
        assert!(resolve_msrv(&HashMap::new(), &Unused).is_err());
        assert!(resolve_msrv(&params("", None), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_msrv(&params("../etc", None), &Unused).is_err());
    }

    #[test]
    fn errors_when_rust_version_is_missing() {
        let fetcher = FakeFetcher(
            "https://crates.io/api/v1/crates/serde/1.0.194",
            r#"{"version": {"num": "1.0.194"}}"#,
        );
        assert!(resolve_msrv(&params("serde", Some("1.0.194")), &fetcher).is_err());
    }
}
