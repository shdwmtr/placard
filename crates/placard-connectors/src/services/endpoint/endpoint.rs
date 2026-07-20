use crate::Fetcher;
use crate::json;
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

pub(crate) fn resolve_endpoint(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("endpoint requires a data-url attribute")?;
    let url = validate_data_url(url)?;

    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "endpoint response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let message = value
        .get("message")
        .ok_or("endpoint response missing message")?;
    message
        .as_text()
        .ok_or_else(|| "message was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://example.com/badge.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(url: &str) -> HashMap<String, String> {
        HashMap::from([("url".to_string(), url.to_string())])
    }

    #[test]
    fn extracts_message_from_an_endpoint_shaped_response() {
        let fetcher =
            FakeFetcher(r#"{"schemaVersion": 1, "label": "hello", "message": "sweet world"}"#);
        let value = resolve_endpoint(&params("https://example.com/badge.json"), &fetcher).unwrap();
        assert_eq!(value, "sweet world");
    }

    #[test]
    fn requires_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_endpoint(&HashMap::new(), &Unused).is_err());
        assert!(resolve_endpoint(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        assert!(resolve_endpoint(&params("file:///etc/passwd"), &Unused).is_err());
        assert!(resolve_endpoint(&params("javascript:alert(1)"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"schemaVersion": 1, "label": "hello"}"#);
        assert!(resolve_endpoint(&params("https://example.com/badge.json"), &fetcher).is_err());
    }
}
