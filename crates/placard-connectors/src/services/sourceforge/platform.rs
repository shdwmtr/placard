use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_platform(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("sourceforge-platform requires a data-project attribute")?;
    let project = validate_path_param("project", project)?;

    let url = format!("https://sourceforge.net/rest/p/{project}/");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourceforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let os = value
        .get("categories.os")
        .ok_or("sourceforge response missing categories.os")?;
    let Value::Array(items) = os else {
        return Err("sourceforge categories.os was not a JSON array".to_string());
    };

    let platforms: Vec<String> = items
        .iter()
        .filter_map(|item| item.get("fullname"))
        .filter_map(|v| v.as_text())
        .collect();

    if platforms.is_empty() {
        return Err("sourceforge categories.os had no platforms with a fullname".to_string());
    }

    Ok(platforms.join(" | "))
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
    fn joins_platform_fullnames_with_a_pipe() {
        let fetcher =
            FakeFetcher(r#"{"categories": {"os": [{"fullname": "Linux"}, {"fullname": "Mac"}]}}"#);
        let value = resolve_platform(&params("guitarix"), &fetcher).unwrap();
        assert_eq!(value, "Linux | Mac");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_platform(&HashMap::new(), &Unused).is_err());
        assert!(resolve_platform(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_platform(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_categories_os_is_missing() {
        let fetcher = FakeFetcher(r#"{"categories": {}}"#);
        assert!(resolve_platform(&params("guitarix"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_categories_os_is_empty() {
        let fetcher = FakeFetcher(r#"{"categories": {"os": []}}"#);
        assert!(resolve_platform(&params("guitarix"), &fetcher).is_err());
    }
}
