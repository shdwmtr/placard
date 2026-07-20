use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let mod_name = params
        .get("mod-name")
        .ok_or("factorio-mod-portal-downloads requires a data-mod-name attribute")?;
    let mod_name = validate_path_param("mod-name", mod_name)?;

    let url = format!("https://mods.factorio.com/api/mods/{mod_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "factorio mod portal response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("downloads_count")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "factorio mod portal response missing downloads_count".to_string())
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

    fn params(mod_name: &str) -> HashMap<String, String> {
        HashMap::from([("mod-name".to_string(), mod_name.to_string())])
    }

    #[test]
    fn extracts_the_downloads_count() {
        let fetcher = FakeFetcher {
            expected_url: "https://mods.factorio.com/api/mods/rso-mod",
            body: r#"{"downloads_count": 12345, "releases": [{"version": "1.0.0", "released_at": "2020-01-01T00:00:00.000Z", "info_json": {"factorio_version": "1.0"}}]}"#,
        };
        let value = resolve_downloads(&params("rso-mod"), &fetcher).unwrap();
        assert_eq!(value, "12345");
    }

    #[test]
    fn requires_a_mod_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid mod-name")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_mod_name() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid mod-name")
            }
        }
        assert!(resolve_downloads(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_downloads_count_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://mods.factorio.com/api/mods/rso-mod",
            body: r#"{"releases": []}"#,
        };
        assert!(resolve_downloads(&params("rso-mod"), &fetcher).is_err());
    }
}
