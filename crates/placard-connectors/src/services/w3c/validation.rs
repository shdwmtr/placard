use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

fn validate_parser(value: &str) -> Result<&str, String> {
    match value {
        "default" | "html" | "xml" | "xmldtd" => Ok(value),
        other => Err(format!(
            "'parser' parameter '{other}' is not one of default, html, xml, xmldtd"
        )),
    }
}

pub(crate) fn resolve_validation(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let target_url = params
        .get("target-url")
        .ok_or("w3c-validation requires a data-target-url attribute")?;
    if target_url.is_empty() {
        return Err("'target-url' parameter must not be empty".to_string());
    }
    let parser = match params.get("parser") {
        Some(value) => validate_parser(value)?,
        None => "default",
    };

    let mut url = format!(
        "https://validator.nu/?doc={}&out=json",
        percent_encode(target_url)
    );
    if parser != "default" {
        url.push_str("&parser=");
        url.push_str(parser);
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "w3c response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let messages = value
        .get("messages")
        .ok_or("w3c response missing messages")?;
    let Value::Array(items) = messages else {
        return Err("w3c response 'messages' was not an array".to_string());
    };

    if items.is_empty() {
        return Ok("validated".to_string());
    }

    let mut errors = 0u32;
    let mut warnings = 0u32;
    for item in items {
        let message_type = item
            .get("type")
            .and_then(|v| v.as_text())
            .ok_or("w3c response message missing type")?;
        if message_type == "info" {
            warnings += 1;
        } else {
            errors += 1;
        }
    }

    let mut parts = Vec::new();
    if errors > 0 {
        parts.push(format!(
            "{errors} error{}",
            if errors > 1 { "s" } else { "" }
        ));
    }
    if warnings > 0 {
        parts.push(format!(
            "{warnings} warning{}",
            if warnings > 1 { "s" } else { "" }
        ));
    }
    Ok(parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://validator.nu/?doc=https%3A%2F%2Fexample.com%2Fpage&out=json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(target_url: &str) -> HashMap<String, String> {
        HashMap::from([("target-url".to_string(), target_url.to_string())])
    }

    #[test]
    fn returns_validated_when_there_are_no_messages() {
        let fetcher = FakeFetcher(r#"{"url": "https://example.com/page", "messages": []}"#);
        let value = resolve_validation(&params("https://example.com/page"), &fetcher).unwrap();
        assert_eq!(value, "validated");
    }

    #[test]
    fn counts_errors_and_warnings_from_messages() {
        let fetcher = FakeFetcher(
            r#"{"url": "https://example.com/page", "messages": [
                {"type": "error", "message": "bad markup"},
                {"type": "error", "message": "worse markup"},
                {"type": "info", "message": "consider this"}
            ]}"#,
        );
        let value = resolve_validation(&params("https://example.com/page"), &fetcher).unwrap();
        assert_eq!(value, "2 errors, 1 warning");
    }

    #[test]
    fn treats_non_document_error_as_an_error() {
        struct NonDocFetcher;
        impl Fetcher for NonDocFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Ok(br#"{"messages": [{"type": "non-document-error", "subType": "io", "message": "boom"}]}"#.to_vec())
            }
        }
        let value =
            resolve_validation(&params("https://example.com/page"), &NonDocFetcher).unwrap();
        assert_eq!(value, "1 error");
    }

    #[test]
    fn requires_target_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a target-url")
            }
        }
        assert!(resolve_validation(&HashMap::new(), &Unused).is_err());
        assert!(resolve_validation(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_parser_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid parser")
            }
        }
        let mut p = params("https://example.com/page");
        p.insert("parser".to_string(), "weird".to_string());
        assert!(resolve_validation(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_messages_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"url": "https://example.com/page"}"#);
        assert!(resolve_validation(&params("https://example.com/page"), &fetcher).is_err());
    }
}
