use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn resolve_base_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("gitea-url") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'gitea-url' parameter must not be empty".to_string());
            }
            if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
                return Err("'gitea-url' parameter must be an http(s) URL".to_string());
            }
            if trimmed
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
            {
                return Err("'gitea-url' parameter contains disallowed characters".to_string());
            }
            Ok(trimmed.to_string())
        }
        None => Ok("https://gitea.com".to_string()),
    }
}

pub(crate) fn resolve_forks(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("gitea-forks requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("gitea-forks requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let base_url = resolve_base_url(params)?;

    let url = format!("{base_url}/api/v1/repos/{user}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitea response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("forks_count")
        .ok_or("gitea response missing forks_count")?;
    count
        .as_text()
        .ok_or_else(|| "forks_count was not a plain value".to_string())
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
    fn extracts_forks_count_from_a_gitea_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea",
            body: r#"{"id": 1, "forks_count": 17}"#,
        };
        let value = resolve_forks(&params("gitea", "tea"), &fetcher).unwrap();
        assert_eq!(value, "17");
    }

    #[test]
    fn uses_a_custom_gitea_url_when_provided() {
        let mut p = params("shdwmtr", "placard");
        p.insert("gitea-url".to_string(), "https://codeberg.org/".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://codeberg.org/api/v1/repos/shdwmtr/placard",
            body: r#"{"forks_count": 2}"#,
        };
        let value = resolve_forks(&p, &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_forks(&HashMap::new(), &Unused).is_err());
        assert!(resolve_forks(&params("gitea", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_forks(&params("../etc", "tea"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea",
            body: r#"{"id": 1}"#,
        };
        assert!(resolve_forks(&params("gitea", "tea"), &fetcher).is_err());
    }
}
