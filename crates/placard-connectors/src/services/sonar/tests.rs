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

fn parse_count(value: Option<String>) -> f64 {
    value.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0)
}

pub(crate) fn resolve_tests(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let component = params
        .get("component")
        .ok_or("sonar-tests requires a data-component attribute")?;
    let component = validate_component(component)?;
    let server = params
        .get("server")
        .ok_or("sonar-tests requires a data-server attribute")?;
    let server = validate_server(server)?;

    let mut url = format!(
        "{server}/api/measures/component?component={}&metricKeys=tests,test_failures,skipped_tests",
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

    let total = find_measure(&value, "tests").ok_or("sonar response missing tests measure")?;
    let total: f64 = total
        .parse()
        .map_err(|_| "tests measure was not numeric".to_string())?;
    let failed = parse_count(find_measure(&value, "test_failures"));
    let skipped = parse_count(find_measure(&value, "skipped_tests"));
    let passed = total - (failed + skipped);

    if total == 0.0 {
        return Ok("no tests".to_string());
    }

    let mut parts = vec![format!("{} passed", passed as i64)];
    if failed > 0.0 {
        parts.push(format!("{} failed", failed as i64));
    }
    if skipped > 0.0 {
        parts.push(format!("{} skipped", skipped as i64));
    }
    Ok(parts.join(", "))
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
    fn summarizes_passed_failed_and_skipped_counts() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=michelin_kstreamplify&metricKeys=tests,test_failures,skipped_tests",
            body: r#"{"component": {"measures": [
                {"metric": "tests", "value": "100"},
                {"metric": "test_failures", "value": "3"},
                {"metric": "skipped_tests", "value": "2"}
            ]}}"#,
        };
        let value = resolve_tests(
            &params("michelin_kstreamplify", "https://sonarcloud.io"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "95 passed, 3 failed, 2 skipped");
    }

    #[test]
    fn omits_zero_failed_and_skipped_counts() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=tests,test_failures,skipped_tests",
            body: r#"{"component": {"measures": [{"metric": "tests", "value": "10"}]}}"#,
        };
        let value = resolve_tests(&params("comp", "https://sonarcloud.io"), &fetcher).unwrap();
        assert_eq!(value, "10 passed");
    }

    #[test]
    fn reports_no_tests_when_total_is_zero() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=tests,test_failures,skipped_tests",
            body: r#"{"component": {"measures": [{"metric": "tests", "value": "0"}]}}"#,
        };
        let value = resolve_tests(&params("comp", "https://sonarcloud.io"), &fetcher).unwrap();
        assert_eq!(value, "no tests");
    }

    #[test]
    fn appends_branch_to_the_query_when_provided() {
        let mut p = params("comp", "https://sonarcloud.io");
        p.insert("branch".to_string(), "main".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=tests,test_failures,skipped_tests&branch=main",
            body: r#"{"component": {"measures": [{"metric": "tests", "value": "5"}]}}"#,
        };
        let value = resolve_tests(&p, &fetcher).unwrap();
        assert_eq!(value, "5 passed");
    }

    #[test]
    fn requires_component_and_server_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_tests(&HashMap::new(), &Unused).is_err());
        assert!(resolve_tests(&params("", "https://sonarcloud.io"), &Unused).is_err());
        assert!(resolve_tests(&params("comp", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_disallowed_component_and_server_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with invalid params")
            }
        }
        assert!(resolve_tests(&params("a?b=c", "https://sonarcloud.io"), &Unused).is_err());
        assert!(resolve_tests(&params("comp", "not-a-url"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_tests_measure_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=tests,test_failures,skipped_tests",
            body: r#"{"component": {"measures": []}}"#,
        };
        assert!(resolve_tests(&params("comp", "https://sonarcloud.io"), &fetcher).is_err());
    }
}
