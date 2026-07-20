use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_crate(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let crate_name = params
        .get("crate")
        .ok_or("deps-rs-crate requires a data-crate attribute")?;
    let crate_name = validate_path_param("crate", crate_name)?;
    let version = params
        .get("version")
        .ok_or("deps-rs-crate requires a data-version attribute")?;
    let version = validate_path_param("version", version)?;

    let url = format!("https://deps.rs/crate/{crate_name}/{version}/shield.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "deps.rs response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let message = value
        .get("message")
        .ok_or("deps.rs response missing message")?;
    message
        .as_text()
        .ok_or_else(|| "message was not a plain value".to_string())
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

    fn params(crate_name: &str, version: &str) -> HashMap<String, String> {
        HashMap::from([
            ("crate".to_string(), crate_name.to_string()),
            ("version".to_string(), version.to_string()),
        ])
    }

    #[test]
    fn extracts_the_message_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://deps.rs/crate/syn/latest/shield.json",
            body: r#"{"message": "up to date"}"#,
        };
        let value = resolve_crate(&params("syn", "latest"), &fetcher).unwrap();
        assert_eq!(value, "up to date");
    }

    #[test]
    fn requires_crate_and_version_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_crate(&HashMap::new(), &Unused).is_err());
        assert!(resolve_crate(&params("syn", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_crate(&params("../etc", "latest"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://deps.rs/crate/syn/latest/shield.json",
            body: r#"{"other": 1}"#,
        };
        assert!(resolve_crate(&params("syn", "latest"), &fetcher).is_err());
    }
}
