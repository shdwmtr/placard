use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

const METRICS: &[&str] = &[
    "violations",
    "blocker_violations",
    "critical_violations",
    "major_violations",
    "minor_violations",
    "info_violations",
];

fn validate_server(value: &str) -> Result<&str, String> {
    let trimmed = value.trim_end_matches('/');
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

fn validate_metric(value: &str) -> Result<&str, String> {
    if METRICS.contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "'metric' parameter '{value}' is not one of violations, blocker_violations, critical_violations, major_violations, minor_violations, info_violations"
        ))
    }
}

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

pub(crate) fn resolve_violations(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let server = params
        .get("server")
        .ok_or("sonar-violations requires a data-server attribute")?;
    let server = validate_server(server)?;
    let component = params
        .get("component")
        .ok_or("sonar-violations requires a data-component attribute")?;
    let component = validate_path_param("component", component)?;
    let metric = match params.get("metric") {
        Some(value) => validate_metric(value)?,
        None => "violations",
    };

    let mut url = format!(
        "{server}/api/measures/component?component={}&metricKeys={metric}",
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
    let measures = value
        .get("component.measures")
        .ok_or("sonar response missing component.measures")?;
    let Value::Array(items) = measures else {
        return Err("sonar response 'component.measures' was not an array".to_string());
    };
    let measure = items
        .iter()
        .find(|item| item.get("metric").and_then(|v| v.as_text()).as_deref() == Some(metric))
        .ok_or("sonar response did not include the requested metric")?;
    measure
        .get("value")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "sonar measure was missing a value".to_string())
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

    fn params(server: &str, component: &str) -> HashMap<String, String> {
        HashMap::from([
            ("server".to_string(), server.to_string()),
            ("component".to_string(), component.to_string()),
        ])
    }

    #[test]
    fn extracts_the_violations_count_from_a_sonar_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=brave_brave-core&metricKeys=violations",
            body: r#"{"component": {"measures": [{"metric": "violations", "value": "5"}]}}"#,
        };
        let value = resolve_violations(
            &params("https://sonarcloud.io", "brave_brave-core"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "5");
    }

    #[test]
    fn uses_a_custom_metric_and_branch_when_provided() {
        let mut p = params("https://sonarcloud.io/", "brave_brave-core");
        p.insert("metric".to_string(), "critical_violations".to_string());
        p.insert("branch".to_string(), "release/1.0".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=brave_brave-core&metricKeys=critical_violations&branch=release%2F1.0",
            body: r#"{"component": {"measures": [{"metric": "critical_violations", "value": "2"}]}}"#,
        };
        let value = resolve_violations(&p, &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn requires_server_and_component_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_violations(&HashMap::new(), &Unused).is_err());
        assert!(resolve_violations(&params("https://sonarcloud.io", ""), &Unused).is_err());
        assert!(resolve_violations(&params("", "brave_brave-core"), &Unused).is_err());
    }

    #[test]
    fn rejects_an_invalid_metric() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid metric")
            }
        }
        let mut p = params("https://sonarcloud.io", "brave_brave-core");
        p.insert("metric".to_string(), "not_a_real_metric".to_string());
        assert!(resolve_violations(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_component_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid component")
            }
        }
        assert!(
            resolve_violations(&params("https://sonarcloud.io", "../etc/passwd"), &Unused).is_err()
        );
    }

    #[test]
    fn rejects_a_non_http_server() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid server")
            }
        }
        assert!(
            resolve_violations(&params("ftp://sonarcloud.io", "brave_brave-core"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_requested_metric_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=brave_brave-core&metricKeys=violations",
            body: r#"{"component": {"measures": []}}"#,
        };
        assert!(
            resolve_violations(
                &params("https://sonarcloud.io", "brave_brave-core"),
                &fetcher
            )
            .is_err()
        );
    }
}
