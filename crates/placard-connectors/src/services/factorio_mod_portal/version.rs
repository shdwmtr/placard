use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let mod_name = params
        .get("mod-name")
        .ok_or("factorio-mod-portal-version requires a data-mod-name attribute")?;
    let mod_name = validate_path_param("mod-name", mod_name)?;

    let url = format!("https://mods.factorio.com/api/mods/{mod_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "factorio mod portal response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let releases = value
        .get("releases")
        .ok_or("factorio mod portal response missing releases")?;
    let json::Value::Array(items) = releases else {
        return Err("factorio mod portal releases was not an array".to_string());
    };
    let latest = items
        .last()
        .ok_or("factorio mod portal releases was empty")?;
    latest
        .get("version")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "factorio mod portal release missing version".to_string())
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
    fn extracts_the_latest_release_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://mods.factorio.com/api/mods/rso-mod",
            body: r#"{"downloads_count": 5, "releases": [
                {"version": "1.0.0", "released_at": "2020-01-01T00:00:00.000Z", "info_json": {"factorio_version": "1.0"}},
                {"version": "2.0.0", "released_at": "2021-01-01T00:00:00.000Z", "info_json": {"factorio_version": "1.1"}}
            ]}"#,
        };
        let value = resolve_version(&params("rso-mod"), &fetcher).unwrap();
        assert_eq!(value, "2.0.0");
    }

    #[test]
    fn requires_a_mod_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid mod-name")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_mod_name() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid mod-name")
            }
        }
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_releases_are_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://mods.factorio.com/api/mods/rso-mod",
            body: r#"{"downloads_count": 5}"#,
        };
        assert!(resolve_version(&params("rso-mod"), &fetcher).is_err());
    }
}
