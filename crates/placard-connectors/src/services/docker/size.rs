use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn normalize_user(user: &str) -> &str {
    if user == "_" { "library" } else { user }
}

pub(crate) fn resolve_size(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("docker-size requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("docker-size requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let user = normalize_user(user);
    let tag = match params.get("tag") {
        Some(tag) => Some(validate_path_param("tag", tag)?),
        None => None,
    };

    let (url, single_entry) = match tag {
        Some(tag) => (
            format!("https://registry.hub.docker.com/v2/repositories/{user}/{repo}/tags/{tag}"),
            true,
        ),
        None => (
            format!(
                "https://registry.hub.docker.com/v2/repositories/{user}/{repo}/tags?page_size=100&ordering=last_updated"
            ),
            false,
        ),
    };

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "docker hub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let entry = if single_entry {
        &value
    } else {
        let results = match value.get("results") {
            Some(Value::Array(items)) => items,
            _ => return Err("docker hub response missing results array".to_string()),
        };
        results.first().ok_or("no tags found")?
    };

    let full_size = entry
        .get("full_size")
        .ok_or("docker hub response missing full_size")?;
    full_size
        .as_text()
        .ok_or_else(|| "full_size was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://registry.hub.docker.com/v2/repositories/library/ubuntu/tags?page_size=100&ordering=last_updated"
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
    fn extracts_full_size_of_the_most_recently_updated_tag() {
        let fetcher = FakeFetcher(
            r#"{"count": 2, "results": [{"name": "latest", "full_size": 29070540}, {"name": "old", "full_size": 1}]}"#,
        );
        let value = resolve_size(&params("_", "ubuntu"), &fetcher).unwrap();
        assert_eq!(value, "29070540");
    }

    #[test]
    fn uses_the_direct_tag_endpoint_when_a_tag_is_given() {
        struct AssertFetcher;
        impl Fetcher for AssertFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://registry.hub.docker.com/v2/repositories/library/ubuntu/tags/22.04"
                );
                Ok(r#"{"name": "22.04", "full_size": 77000000}"#.as_bytes().to_vec())
            }
        }
        let mut p = params("_", "ubuntu");
        p.insert("tag".to_string(), "22.04".to_string());
        let value = resolve_size(&p, &AssertFetcher).unwrap();
        assert_eq!(value, "77000000");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_size(&HashMap::new(), &Unused).is_err());
        assert!(resolve_size(&params("_", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_size(&params("_", "../etc"), &Unused).is_err());
        let mut p = params("_", "ubuntu");
        p.insert("tag".to_string(), "../etc".to_string());
        assert!(resolve_size(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"count": 1, "results": [{"name": "latest"}]}"#);
        assert!(resolve_size(&params("_", "ubuntu"), &fetcher).is_err());
    }
}
