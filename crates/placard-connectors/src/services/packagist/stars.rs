use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn resolve_server_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("server") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
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
            Ok(trimmed.to_string())
        }
        None => Ok("https://packagist.org".to_string()),
    }
}

pub(crate) fn resolve_stars(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("packagist-stars requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("packagist-stars requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let server = resolve_server_url(params)?;

    let url = format!(
        "{server}/packages/{}/{}.json",
        user.to_lowercase(),
        repo.to_lowercase()
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "packagist response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let favers = value
        .get("package.favers")
        .ok_or("packagist response missing package.favers")?;
    favers
        .as_text()
        .ok_or_else(|| "package.favers was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://packagist.org/packages/guzzlehttp/guzzle.json");
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
    fn extracts_favers_from_a_packagist_shaped_response() {
        let fetcher = FakeFetcher(r#"{"package": {"name": "guzzlehttp/guzzle", "favers": 22222}}"#);
        let value = resolve_stars(&params("guzzlehttp", "Guzzle"), &fetcher).unwrap();
        assert_eq!(value, "22222");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_stars(&HashMap::new(), &Unused).is_err());
        assert!(resolve_stars(&params("guzzlehttp", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_stars(&params("../etc", "guzzle"), &Unused).is_err());
    }

    #[test]
    fn errors_when_favers_is_missing() {
        let fetcher = FakeFetcher(r#"{"package": {"name": "guzzlehttp/guzzle"}}"#);
        assert!(resolve_stars(&params("guzzlehttp", "guzzle"), &fetcher).is_err());
    }
}
