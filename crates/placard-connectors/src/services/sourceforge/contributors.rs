use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_contributors(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("sourceforge-contributors requires a data-project attribute")?;
    let project = validate_path_param("project", project)?;

    let url = format!("https://sourceforge.net/rest/p/{project}/");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourceforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    match value.get("developers") {
        Some(Value::Array(items)) => Ok(items.len().to_string()),
        _ => Err("sourceforge response missing developers array".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://sourceforge.net/rest/p/guitarix/");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(project: &str) -> HashMap<String, String> {
        HashMap::from([("project".to_string(), project.to_string())])
    }

    #[test]
    fn counts_the_developers_array() {
        let fetcher = FakeFetcher(r#"{"developers": [{"name": "a"}, {"name": "b"}]}"#);
        let value = resolve_contributors(&params("guitarix"), &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project param")
            }
        }
        assert!(resolve_contributors(&HashMap::new(), &Unused).is_err());
        assert!(resolve_contributors(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_contributors(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_developers_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "guitarix"}"#);
        assert!(resolve_contributors(&params("guitarix"), &fetcher).is_err());
    }
}
