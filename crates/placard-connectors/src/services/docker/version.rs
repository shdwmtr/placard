use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn normalize_user(user: &str) -> &str {
    if user == "_" { "library" } else { user }
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("docker-version requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("docker-version requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let user = normalize_user(user);

    let url = format!(
        "https://registry.hub.docker.com/v2/repositories/{user}/{repo}/tags?page_size=100&ordering=last_updated"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "docker hub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let results = match value.get("results") {
        Some(Value::Array(items)) => items,
        _ => return Err("docker hub response missing results array".to_string()),
    };
    let latest = results.first().ok_or("no tags found")?;
    latest
        .get("name")
        .ok_or("docker hub tag entry missing name")?
        .as_text()
        .ok_or_else(|| "tag name was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://registry.hub.docker.com/v2/repositories/library/alpine/tags?page_size=100&ordering=last_updated"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_name_of_the_most_recently_updated_tag() {
        let fetcher =
            FakeFetcher(r#"{"count": 2, "results": [{"name": "3.19"}, {"name": "3.18"}]}"#);
        let value = resolve_version(&params("_", "alpine"), &fetcher).unwrap();
        assert_eq!(value, "3.19");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("_", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc", "alpine"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_tags() {
        let fetcher = FakeFetcher(r#"{"count": 0, "results": []}"#);
        assert!(resolve_version(&params("_", "alpine"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_name_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"count": 1, "results": [{"images": []}]}"#);
        assert!(resolve_version(&params("_", "alpine"), &fetcher).is_err());
    }
}
