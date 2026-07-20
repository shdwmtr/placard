use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_tag(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-tag requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-tag requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/tags");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(tags) = value else {
        return Err("github response was not a JSON array".to_string());
    };
    let latest = tags.first().ok_or("no tags found")?;
    latest
        .get("name")
        .ok_or("github tag entry missing name")?
        .as_text()
        .ok_or_else(|| "tag name was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.github.com/repos/expressjs/express/tags");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_name_of_the_first_tag() {
        let fetcher = FakeFetcher(r#"[{"name": "5.0.1"}, {"name": "5.0.0"}, {"name": "4.18.2"}]"#);
        let value = resolve_tag(&params("expressjs", "express"), &fetcher).unwrap();
        assert_eq!(value, "5.0.1");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_tag(&HashMap::new(), &Unused).is_err());
        assert!(resolve_tag(&params("expressjs", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_tag(&params("../etc", "express"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_tags() {
        let fetcher = FakeFetcher("[]");
        assert!(resolve_tag(&params("expressjs", "express"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_name_field_is_missing() {
        let fetcher = FakeFetcher(r#"[{"commit": {"sha": "abc"}}]"#);
        assert!(resolve_tag(&params("expressjs", "express"), &fetcher).is_err());
    }
}
