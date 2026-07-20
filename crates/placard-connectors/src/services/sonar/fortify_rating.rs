use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_component(component: &str) -> Result<&str, String> {
    if component.is_empty() {
        return Err("'component' parameter must not be empty".to_string());
    }
    if !component
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
    {
        return Err("'component' parameter contains disallowed characters".to_string());
    }
    Ok(component)
}

fn validate_server(server: &str) -> Result<String, String> {
    if server.is_empty() {
        return Err("'server' parameter must not be empty".to_string());
    }
    let trimmed = server.trim_end_matches('/');
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

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn find_measure(value: &json::Value, metric_key: &str) -> Option<String> {
    let measures = value.get("component.measures")?;
    let json::Value::Array(items) = measures else {
        return None;
    };
    for item in items {
        if item.get("metric").and_then(|v| v.as_text()).as_deref() == Some(metric_key) {
            return item.get("value").and_then(|v| v.as_text());
        }
    }
    None
}

pub(crate) fn resolve_fortify_rating(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let component = params
        .get("component")
        .ok_or("sonar-fortify-rating requires a data-component attribute")?;
    let component = validate_component(component)?;
    let server = params
        .get("server")
        .ok_or("sonar-fortify-rating requires a data-server attribute")?;
    let server = validate_server(server)?;

    let mut url = format!(
        "{server}/api/measures/component?component={}&metricKeys=fortify-security-rating",
        percent_encode(component)
    );
    if let Some(branch) = params.get("branch") {
        url.push_str("&branch=");
        url.push_str(&percent_encode(branch));
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "sonar response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let rating = find_measure(&value, "fortify-security-rating")
        .ok_or("sonar response missing fortify-security-rating measure")?;
    Ok(format!("{rating}/5"))
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

    fn params(component: &str, server: &str) -> HashMap<String, String> {
        HashMap::from([
            ("component".to_string(), component.to_string()),
            ("server".to_string(), server.to_string()),
        ])
    }

    #[test]
    fn extracts_fortify_rating_from_measures() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonar.example.com/api/measures/component?component=michelin_kstreamplify&metricKeys=fortify-security-rating",
            body: r#"{"component": {"measures": [{"metric": "fortify-security-rating", "value": "4"}]}}"#,
        };
        let value = resolve_fortify_rating(
            &params("michelin_kstreamplify", "https://sonar.example.com"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "4/5");
    }

    #[test]
    fn appends_branch_to_the_query_when_provided() {
        let mut p = params("michelin_kstreamplify", "https://sonar.example.com");
        p.insert("branch".to_string(), "main".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://sonar.example.com/api/measures/component?component=michelin_kstreamplify&metricKeys=fortify-security-rating&branch=main",
            body: r#"{"component": {"measures": [{"metric": "fortify-security-rating", "value": "5"}]}}"#,
        };
        let value = resolve_fortify_rating(&p, &fetcher).unwrap();
        assert_eq!(value, "5/5");
    }

    #[test]
    fn requires_component_and_server_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_fortify_rating(&HashMap::new(), &Unused).is_err());
        assert!(resolve_fortify_rating(&params("", "https://sonar.example.com"), &Unused).is_err());
        assert!(resolve_fortify_rating(&params("comp", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_disallowed_component_and_server_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with invalid params")
            }
        }
        assert!(
            resolve_fortify_rating(&params("a?b=c", "https://sonar.example.com"), &Unused).is_err()
        );
        assert!(resolve_fortify_rating(&params("comp", "not-a-url"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_measure_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonar.example.com/api/measures/component?component=comp&metricKeys=fortify-security-rating",
            body: r#"{"component": {"measures": []}}"#,
        };
        assert!(
            resolve_fortify_rating(&params("comp", "https://sonar.example.com"), &fetcher).is_err()
        );
    }
}
