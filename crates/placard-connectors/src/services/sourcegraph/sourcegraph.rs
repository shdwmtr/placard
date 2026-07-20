use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_segmented_param<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("'{name}' parameter must not be empty"));
    }
    for segment in value.split('/') {
        validate_path_param(name, segment)?;
    }
    Ok(value)
}

pub(crate) fn resolve_sourcegraph(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let repo = params
        .get("repo")
        .ok_or("sourcegraph requires a data-repo attribute")?;
    let repo = validate_segmented_param("repo", repo)?;

    let url = format!("https://sourcegraph.com/.api/repos/{repo}/-/shield");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourcegraph response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let raw = value
        .get("value")
        .ok_or("sourcegraph response missing value")?;
    let text = raw
        .as_text()
        .ok_or_else(|| "value was not a plain value".to_string())?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://sourcegraph.com/.api/repos/github.com/gorilla/mux/-/shield"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(repo: &str) -> HashMap<String, String> {
        HashMap::from([("repo".to_string(), repo.to_string())])
    }

    #[test]
    fn extracts_the_trimmed_projects_count() {
        let fetcher = FakeFetcher(r#"{"value": " 123 projects"}"#);
        let value = resolve_sourcegraph(&params("github.com/gorilla/mux"), &fetcher).unwrap();
        assert_eq!(value, "123 projects");
    }

    #[test]
    fn requires_a_repo_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_sourcegraph(&HashMap::new(), &Unused).is_err());
        assert!(resolve_sourcegraph(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_sourcegraph(&params("github.com/a?b=c"), &Unused).is_err());
        assert!(resolve_sourcegraph(&params("github.com//mux"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"other": 1}"#);
        assert!(resolve_sourcegraph(&params("github.com/gorilla/mux"), &fetcher).is_err());
    }
}
