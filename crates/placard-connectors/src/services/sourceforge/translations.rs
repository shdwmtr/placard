use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_translations(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("sourceforge-translations requires a data-project attribute")?;
    let project = validate_path_param("project", project)?;

    let url = format!("https://sourceforge.net/rest/p/{project}/");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourceforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let translation = value
        .get("categories.translation")
        .ok_or("sourceforge response missing categories.translation")?;
    let Value::Array(items) = translation else {
        return Err("sourceforge categories.translation was not a JSON array".to_string());
    };

    Ok(items.len().to_string())
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
    fn counts_translation_entries() {
        let fetcher = FakeFetcher(r#"{"categories": {"translation": ["fr", "de", "es"]}}"#);
        let value = resolve_translations(&params("guitarix"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn returns_zero_for_no_translations() {
        let fetcher = FakeFetcher(r#"{"categories": {"translation": []}}"#);
        let value = resolve_translations(&params("guitarix"), &fetcher).unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_translations(&HashMap::new(), &Unused).is_err());
        assert!(resolve_translations(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_translations(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_categories_translation_is_missing() {
        let fetcher = FakeFetcher(r#"{"categories": {}}"#);
        assert!(resolve_translations(&params("guitarix"), &fetcher).is_err());
    }
}
