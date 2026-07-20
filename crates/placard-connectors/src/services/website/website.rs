use crate::Fetcher;
use std::collections::HashMap;

fn validate_data_url(url: &str) -> Result<&str, String> {
    if url.is_empty() {
        return Err("'url' parameter must not be empty".to_string());
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("'url' parameter must be a well-formed http:// or https:// URL".to_string());
    }
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(
            "'url' parameter contains disallowed whitespace or control characters".to_string(),
        );
    }
    Ok(url)
}

pub(crate) fn resolve_website(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("website requires a data-url attribute")?;
    let url = validate_data_url(url)?;

    let up_message = params.get("up_message").map(String::as_str).unwrap_or("up");
    let down_message = params
        .get("down_message")
        .map(String::as_str)
        .unwrap_or("down");

    Ok(match fetcher.fetch(url) {
        Ok(_) => up_message.to_string(),
        Err(_) => down_message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SucceedingFetcher;
    impl Fetcher for SucceedingFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://shields.io");
            Ok(Vec::new())
        }
    }

    struct FailingFetcher;
    impl Fetcher for FailingFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://shields.io");
            Err("connection refused".to_string())
        }
    }

    fn params(url: &str) -> HashMap<String, String> {
        HashMap::from([("url".to_string(), url.to_string())])
    }

    #[test]
    fn reports_up_when_the_fetch_succeeds() {
        let value = resolve_website(&params("https://shields.io"), &SucceedingFetcher).unwrap();
        assert_eq!(value, "up");
    }

    #[test]
    fn reports_down_when_the_fetch_fails() {
        let value = resolve_website(&params("https://shields.io"), &FailingFetcher).unwrap();
        assert_eq!(value, "down");
    }

    #[test]
    fn honors_custom_up_and_down_messages() {
        let mut p = params("https://shields.io");
        p.insert("up_message".to_string(), "online".to_string());
        let value = resolve_website(&p, &SucceedingFetcher).unwrap();
        assert_eq!(value, "online");

        let mut p = params("https://shields.io");
        p.insert("down_message".to_string(), "offline".to_string());
        let value = resolve_website(&p, &FailingFetcher).unwrap();
        assert_eq!(value, "offline");
    }

    #[test]
    fn requires_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_website(&HashMap::new(), &Unused).is_err());
        assert!(resolve_website(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        assert!(resolve_website(&params("file:///etc/passwd"), &Unused).is_err());
        assert!(resolve_website(&params("javascript:alert(1)"), &Unused).is_err());
    }
}
