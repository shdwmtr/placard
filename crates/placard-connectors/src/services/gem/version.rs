use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let gem = params
        .get("gem")
        .ok_or("gem-version requires a data-gem attribute")?;
    let gem = validate_path_param("gem", gem)?;

    if params.contains_key("include_prereleases") {
        let url = format!("https://rubygems.org/api/v1/versions/{gem}.json");
        let bytes = fetcher.fetch(&url)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| "rubygems response was not valid UTF-8".to_string())?;
        let value = json::parse(&text)?;
        let Value::Array(versions) = value else {
            return Err("rubygems response was not an array".to_string());
        };
        let first = versions.first().ok_or("no released version found")?;
        return first
            .get("number")
            .and_then(|v| v.as_text())
            .ok_or_else(|| "version entry missing number".to_string());
    }

    let url = format!("https://rubygems.org/api/v1/gems/{gem}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "rubygems response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("version")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "rubygems response missing version".to_string())
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

    fn params(gem: &str) -> HashMap<String, String> {
        HashMap::from([("gem".to_string(), gem.to_string())])
    }

    #[test]
    fn extracts_the_current_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/gems/formatador.json",
            body: r#"{"version": "1.1.0"}"#,
        };
        let value = resolve_version(&params("formatador"), &fetcher).unwrap();
        assert_eq!(value, "1.1.0");
    }

    #[test]
    fn includes_prereleases_when_requested() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/versions/formatador.json",
            body: r#"[{"number": "1.2.0.rc1"}, {"number": "1.1.0"}]"#,
        };
        let mut p = params("formatador");
        p.insert("include_prereleases".to_string(), String::new());
        let value = resolve_version(&p, &fetcher).unwrap();
        assert_eq!(value, "1.2.0.rc1");
    }

    #[test]
    fn requires_gem_param() {
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
        assert!(resolve_version(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://rubygems.org/api/v1/gems/formatador.json",
            body: r#"{"id": 1}"#,
        };
        assert!(resolve_version(&params("formatador"), &fetcher).is_err());
    }
}
