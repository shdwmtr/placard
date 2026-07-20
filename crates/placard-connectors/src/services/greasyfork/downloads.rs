use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn fetch_script(script_id: &str, fetcher: &dyn Fetcher) -> Result<json::Value, String> {
    let primary = format!("https://greasyfork.org/scripts/{script_id}.json");
    let text = match fetcher.fetch(&primary) {
        Ok(bytes) => String::from_utf8(bytes)
            .map_err(|_| "greasyfork response was not valid UTF-8".to_string())?,
        Err(_) => {
            let fallback = format!("https://sleazyfork.org/scripts/{script_id}.json");
            let bytes = fetcher.fetch(&fallback)?;
            String::from_utf8(bytes)
                .map_err(|_| "sleazyfork response was not valid UTF-8".to_string())?
        }
    };
    json::parse(&text)
}

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let variant = params
        .get("variant")
        .ok_or("greasyfork-downloads requires a data-variant attribute")?;
    if !matches!(variant.as_str(), "dt" | "dd") {
        return Err("'variant' parameter must be one of dt, dd".to_string());
    }
    let script_id = params
        .get("script-id")
        .ok_or("greasyfork-downloads requires a data-script-id attribute")?;
    let script_id = validate_path_param("script-id", script_id)?;

    let value = fetch_script(script_id, fetcher)?;
    let field = if variant == "dd" {
        "daily_installs"
    } else {
        "total_installs"
    };
    value
        .get(field)
        .and_then(|v| v.as_text())
        .ok_or_else(|| format!("greasyfork response missing {field}"))
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

    fn params(variant: &str, script_id: &str) -> HashMap<String, String> {
        HashMap::from([
            ("variant".to_string(), variant.to_string()),
            ("script-id".to_string(), script_id.to_string()),
        ])
    }

    #[test]
    fn extracts_total_installs() {
        let fetcher = FakeFetcher {
            expected_url: "https://greasyfork.org/scripts/406540.json",
            body: r#"{"daily_installs": 12, "total_installs": 34567}"#,
        };
        let value = resolve_downloads(&params("dt", "406540"), &fetcher).unwrap();
        assert_eq!(value, "34567");
    }

    #[test]
    fn extracts_daily_installs() {
        let fetcher = FakeFetcher {
            expected_url: "https://greasyfork.org/scripts/406540.json",
            body: r#"{"daily_installs": 12, "total_installs": 34567}"#,
        };
        let value = resolve_downloads(&params("dd", "406540"), &fetcher).unwrap();
        assert_eq!(value, "12");
    }

    #[test]
    fn falls_back_to_sleazyfork_when_greasyfork_fetch_fails() {
        struct FallbackFetcher;
        impl Fetcher for FallbackFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                if url == "https://greasyfork.org/scripts/406540.json" {
                    return Err("not found".to_string());
                }
                assert_eq!(url, "https://sleazyfork.org/scripts/406540.json");
                Ok(br#"{"total_installs": 99}"#.to_vec())
            }
        }
        let value = resolve_downloads(&params("dt", "406540"), &FallbackFetcher).unwrap();
        assert_eq!(value, "99");
    }

    #[test]
    fn requires_variant_and_script_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("dt", ""), &Unused).is_err());
        assert!(resolve_downloads(&params("bogus", "406540"), &Unused).is_err());
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
            expected_url: "https://greasyfork.org/scripts/406540.json",
            body: r#"{"id": 1}"#,
        };
        assert!(resolve_downloads(&params("dt", "406540"), &fetcher).is_err());
    }
}
