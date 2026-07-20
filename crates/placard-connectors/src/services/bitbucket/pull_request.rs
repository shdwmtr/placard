use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_server(server: &str) -> Result<&str, String> {
    let trimmed = server.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("'server' parameter must not be empty".to_string());
    }
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err("'server' parameter must be an http(s) URL".to_string());
    }
    if trimmed
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
    {
        return Err("'server' parameter contains disallowed characters".to_string());
    }
    Ok(trimmed)
}

/// When `data-server` is omitted this hits Bitbucket Cloud; when provided it
/// hits a self-hosted Bitbucket Server instance instead. Both API shapes
/// expose the same top-level `size` field for open pull requests.
pub(crate) fn resolve_pull_request(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("bitbucket-pull-request requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("bitbucket-pull-request requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let url = match params.get("server") {
        Some(server) => {
            let server = validate_server(server)?;
            format!(
                "{server}/rest/api/1.0/projects/{user}/repos/{repo}/pull-requests?state=OPEN&limit=100&withProperties=false&withAttributes=false"
            )
        }
        None => format!(
            "https://bitbucket.org/api/2.0/repositories/{user}/{repo}/pullrequests/?state=OPEN&limit=0"
        ),
    };

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "bitbucket response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let size = value.get("size").ok_or("bitbucket response missing size")?;
    size.as_text()
        .ok_or_else(|| "size was not a plain value".to_string())
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

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_size_field_from_bitbucket_cloud() {
        let fetcher = FakeFetcher {
            expected_url: "https://bitbucket.org/api/2.0/repositories/shields-io/test-repo/pullrequests/?state=OPEN&limit=0",
            body: r#"{"size": 3}"#,
        };
        let value = resolve_pull_request(&params("shields-io", "test-repo"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn extracts_the_size_field_from_a_bitbucket_server_instance() {
        let mut p = params("shields-io", "test-repo");
        p.insert(
            "server".to_string(),
            "https://bitbucket.mydomain.net".to_string(),
        );
        let fetcher = FakeFetcher {
            expected_url: "https://bitbucket.mydomain.net/rest/api/1.0/projects/shields-io/repos/test-repo/pull-requests?state=OPEN&limit=100&withProperties=false&withAttributes=false",
            body: r#"{"size": 5}"#,
        };
        let value = resolve_pull_request(&p, &fetcher).unwrap();
        assert_eq!(value, "5");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_pull_request(&HashMap::new(), &Unused).is_err());
        assert!(resolve_pull_request(&params("shields-io", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_pull_request(&params("../etc", "test-repo"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_size_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://bitbucket.org/api/2.0/repositories/shields-io/test-repo/pullrequests/?state=OPEN&limit=0",
            body: r#"{"page": 1}"#,
        };
        assert!(resolve_pull_request(&params("shields-io", "test-repo"), &fetcher).is_err());
    }
}
