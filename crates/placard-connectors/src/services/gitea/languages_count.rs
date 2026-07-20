use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
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

pub(crate) fn resolve_languages_count(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("gitea-languages-count requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("gitea-languages-count requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let base_url = resolve_base_url(params)?;

    let url = format!("{base_url}/api/v1/repos/{user}/{repo}/languages");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitea response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Object(fields) = value else {
        return Err("gitea response was not a JSON object".to_string());
    };
    Ok(fields.len().to_string())
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
    fn counts_the_languages_in_the_response_object() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea/languages",
            body: r#"{"Python": 39624, "Shell": 104, "Go": 8000}"#,
        };
        let value = resolve_languages_count(&params("gitea", "tea"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn handles_an_empty_repo() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea/languages",
            body: "{}",
        };
        let value = resolve_languages_count(&params("gitea", "tea"), &fetcher).unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn uses_a_custom_gitea_url_when_provided() {
        let mut p = params("shdwmtr", "placard");
        p.insert("gitea-url".to_string(), "https://codeberg.org".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://codeberg.org/api/v1/repos/shdwmtr/placard/languages",
            body: r#"{"Rust": 1000}"#,
        };
        let value = resolve_languages_count(&p, &fetcher).unwrap();
        assert_eq!(value, "1");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_languages_count(&HashMap::new(), &Unused).is_err());
        assert!(resolve_languages_count(&params("gitea", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_languages_count(&params("../etc", "tea"), &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_not_an_object() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea/languages",
            body: "[1, 2, 3]",
        };
        assert!(resolve_languages_count(&params("gitea", "tea"), &fetcher).is_err());
    }
}
