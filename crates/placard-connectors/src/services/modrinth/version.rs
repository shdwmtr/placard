use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_id = params
        .get("project-id")
        .ok_or("modrinth-version requires a data-project-id attribute")?;
    let project_id = validate_path_param("project-id", project_id)?;

    let url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "modrinth response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let versions = match &value {
        Value::Array(items) => items,
        _ => return Err("modrinth response was not an array".to_string()),
    };
    let latest = versions.first().ok_or("modrinth project has no versions")?;
    latest
        .get("version_number")
        .ok_or("modrinth version missing version_number")?
        .as_text()
        .ok_or_else(|| "version_number was not a plain value".to_string())
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

    fn params(project_id: &str) -> HashMap<String, String> {
        HashMap::from([("project-id".to_string(), project_id.to_string())])
    }

    #[test]
    fn extracts_the_latest_version_number() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.modrinth.com/v2/project/AANobbMI/version",
            body: r#"[{"version_number": "0.9.1", "game_versions": ["1.21.1"]}, {"version_number": "0.9.0", "game_versions": ["1.21"]}]"#,
        };
        let value = resolve_version(&params("AANobbMI"), &fetcher).unwrap();
        assert_eq!(value, "0.9.1");
    }

    #[test]
    fn requires_a_project_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project id")
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
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_versions() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.modrinth.com/v2/project/AANobbMI/version",
            body: r#"[]"#,
        };
        assert!(resolve_version(&params("AANobbMI"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.modrinth.com/v2/project/AANobbMI/version",
            body: r#"[{"game_versions": ["1.21.1"]}]"#,
        };
        assert!(resolve_version(&params("AANobbMI"), &fetcher).is_err());
    }
}
