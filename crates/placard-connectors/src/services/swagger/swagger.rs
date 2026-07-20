use crate::Fetcher;
use crate::json;
use crate::json::Value;
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

pub(crate) fn resolve_swagger(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let spec_url = params
        .get("spec-url")
        .ok_or("swagger requires a data-spec-url attribute")?;
    if spec_url.is_empty() {
        return Err("'spec-url' parameter must not be empty".to_string());
    }

    let url = format!(
        "https://validator.swagger.io/validator/debug?url={}",
        percent_encode(spec_url)
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "swagger response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let messages = value.get("schemaValidationMessages");
    let is_valid = match messages {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Array(items)) => {
            items.is_empty()
                || items.iter().all(|item| {
                    item.get("level").and_then(|v| v.as_text()).as_deref() == Some("warning")
                })
        }
        Some(_) => {
            return Err("swagger response schemaValidationMessages was not an array".to_string());
        }
    };

    Ok(if is_valid {
        "valid".to_string()
    } else {
        "invalid".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://validator.swagger.io/validator/debug?url=https%3A%2F%2Fexample.com%2Fspec.json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(spec_url: &str) -> HashMap<String, String> {
        HashMap::from([("spec-url".to_string(), spec_url.to_string())])
    }

    #[test]
    fn valid_when_there_are_no_validation_messages() {
        let fetcher = FakeFetcher(r#"{}"#);
        let value = resolve_swagger(&params("https://example.com/spec.json"), &fetcher).unwrap();
        assert_eq!(value, "valid");
    }

    #[test]
    fn valid_when_validation_messages_are_empty() {
        let fetcher = FakeFetcher(r#"{"schemaValidationMessages": []}"#);
        let value = resolve_swagger(&params("https://example.com/spec.json"), &fetcher).unwrap();
        assert_eq!(value, "valid");
    }

    #[test]
    fn valid_when_all_messages_are_warnings() {
        let fetcher = FakeFetcher(
            r#"{"schemaValidationMessages": [{"level": "warning", "message": "minor issue"}]}"#,
        );
        let value = resolve_swagger(&params("https://example.com/spec.json"), &fetcher).unwrap();
        assert_eq!(value, "valid");
    }

    #[test]
    fn invalid_when_any_message_is_an_error() {
        let fetcher = FakeFetcher(
            r#"{"schemaValidationMessages": [
                {"level": "error", "message": "broken schema"},
                {"level": "warning", "message": "minor issue"}
            ]}"#,
        );
        let value = resolve_swagger(&params("https://example.com/spec.json"), &fetcher).unwrap();
        assert_eq!(value, "invalid");
    }

    #[test]
    fn requires_spec_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a spec-url")
            }
        }
        assert!(resolve_swagger(&HashMap::new(), &Unused).is_err());
        assert!(resolve_swagger(&params(""), &Unused).is_err());
    }

    #[test]
    fn errors_on_malformed_response() {
        let fetcher = FakeFetcher(r#"{"schemaValidationMessages": "not-an-array"}"#);
        assert!(resolve_swagger(&params("https://example.com/spec.json"), &fetcher).is_err());
    }
}
