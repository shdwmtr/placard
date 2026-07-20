use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn pick_version_downloads<'a>(versions: &'a [Value], wanted: &str) -> Option<&'a Value> {
    if wanted == "stable" {
        return versions
            .iter()
            .find(|v| v.get("prerelease").map(|p| p.as_text()) == Some(Some("false".to_string())));
    }
    versions
        .iter()
        .find(|v| v.get("number").and_then(|n| n.as_text()).as_deref() == Some(wanted))
}

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let variant = params
        .get("variant")
        .ok_or("gem-downloads requires a data-variant attribute")?;
    if !matches!(variant.as_str(), "dt" | "dtv" | "dv") {
        return Err("'variant' parameter must be one of dt, dtv, dv".to_string());
    }
    let gem = params
        .get("gem")
        .ok_or("gem-downloads requires a data-gem attribute")?;
    let gem = validate_path_param("gem", gem)?;

    if variant == "dv" {
        let version = params
            .get("version")
            .ok_or("gem-downloads requires a data-version attribute when data-variant is dv")?;
        let version = validate_path_param("version", version)?;

        let url = format!("https://rubygems.org/api/v1/versions/{gem}.json");
        let bytes = fetcher.fetch(&url)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| "rubygems response was not valid UTF-8".to_string())?;
        let value = json::parse(&text)?;
        let Value::Array(versions) = value else {
            return Err("rubygems response was not an array".to_string());
        };
        let entry = pick_version_downloads(&versions, version).ok_or("version not found")?;
        return entry
            .get("downloads_count")
            .and_then(|v| v.as_text())
            .ok_or_else(|| "version entry missing downloads_count".to_string());
    }

    let url = format!("https://rubygems.org/api/v1/gems/{gem}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "rubygems response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let field = if variant == "dtv" {
        "version_downloads"
    } else {
        "downloads"
    };
    value
        .get(field)
        .and_then(|v| v.as_text())
        .ok_or_else(|| format!("rubygems response missing {field}"))
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

    fn params(variant: &str, gem: &str) -> HashMap<String, String> {
        HashMap::from([
            ("variant".to_string(), variant.to_string()),
            ("gem".to_string(), gem.to_string()),
        ])
    }

    #[test]
    fn extracts_total_downloads() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/gems/rails.json",
            body: r#"{"downloads": 500000000, "version_downloads": 12345}"#,
        };
        let value = resolve_downloads(&params("dt", "rails"), &fetcher).unwrap();
        assert_eq!(value, "500000000");
    }

    #[test]
    fn extracts_version_downloads_for_latest_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/gems/rails.json",
            body: r#"{"downloads": 500000000, "version_downloads": 12345}"#,
        };
        let value = resolve_downloads(&params("dtv", "rails"), &fetcher).unwrap();
        assert_eq!(value, "12345");
    }

    #[test]
    fn extracts_downloads_for_a_specific_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/versions/rails.json",
            body: r#"[{"number": "7.1.0", "prerelease": false, "downloads_count": 1000}, {"number": "7.0.0", "prerelease": false, "downloads_count": 2000}]"#,
        };
        let mut p = params("dv", "rails");
        p.insert("version".to_string(), "7.0.0".to_string());
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "2000");
    }

    #[test]
    fn resolves_stable_to_first_non_prerelease_entry() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/versions/rails.json",
            body: r#"[{"number": "7.1.0.rc1", "prerelease": true, "downloads_count": 5}, {"number": "7.0.0", "prerelease": false, "downloads_count": 2000}]"#,
        };
        let mut p = params("dv", "rails");
        p.insert("version".to_string(), "stable".to_string());
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "2000");
    }

    #[test]
    fn requires_variant_and_gem_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("dt", ""), &Unused).is_err());
        assert!(resolve_downloads(&params("bogus", "rails"), &Unused).is_err());
    }

    #[test]
    fn requires_version_for_dv_variant() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a version")
            }
        }
        assert!(resolve_downloads(&params("dv", "rails"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("dt", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/gems/rails.json",
            body: r#"{"id": 1}"#,
        };
        assert!(resolve_downloads(&params("dt", "rails"), &fetcher).is_err());
    }
}
