use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn normalize_user(user: &str) -> &str {
    if user == "_" { "library" } else { user }
}

pub(crate) fn resolve_automated(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("docker-automated requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("docker-automated requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let user = normalize_user(user);

    let url = format!("https://registry.hub.docker.com/v2/repositories/{user}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "docker hub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let is_automated = value
        .get("is_automated")
        .ok_or("docker hub response missing is_automated")?;
    is_automated
        .as_text()
        .ok_or_else(|| "is_automated was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://registry.hub.docker.com/v2/repositories/library/ubuntu"
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
    fn extracts_is_automated_from_a_docker_hub_shaped_response() {
        let fetcher = FakeFetcher(r#"{"is_automated": true, "name": "ubuntu"}"#);
        let value = resolve_automated(&params("_", "ubuntu"), &fetcher).unwrap();
        assert_eq!(value, "true");
    }

    #[test]
    fn normalizes_underscore_user_to_library() {
        struct AssertFetcher;
        impl Fetcher for AssertFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://registry.hub.docker.com/v2/repositories/library/ubuntu"
                );
                Ok(r#"{"is_automated": false}"#.as_bytes().to_vec())
            }
        }
        let value = resolve_automated(&params("_", "ubuntu"), &AssertFetcher).unwrap();
        assert_eq!(value, "false");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_automated(&HashMap::new(), &Unused).is_err());
        assert!(resolve_automated(&params("jrottenberg", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_automated(&params("../etc", "ffmpeg"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "ubuntu"}"#);
        assert!(resolve_automated(&params("_", "ubuntu"), &fetcher).is_err());
    }
}
