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

fn validate_metric(metric: &str) -> Result<&str, String> {
    if metric.is_empty() {
        return Err("'metric' parameter must not be empty".to_string());
    }
    if !metric
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("'metric' parameter contains disallowed characters".to_string());
    }
    Ok(metric)
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

pub(crate) fn resolve_generic(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let component = params
        .get("component")
        .ok_or("sonar-generic requires a data-component attribute")?;
    let component = validate_component(component)?;
    let metric = params
        .get("metric")
        .ok_or("sonar-generic requires a data-metric attribute")?;
    let metric = validate_metric(metric)?;
    let server = params
        .get("server")
        .ok_or("sonar-generic requires a data-server attribute")?;
    let server = validate_server(server)?;

    let mut url = format!(
        "{server}/api/measures/component?component={}&metricKeys={}",
        percent_encode(component),
        percent_encode(metric)
    );
    if let Some(branch) = params.get("branch") {
        url.push_str("&branch=");
        url.push_str(&percent_encode(branch));
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "sonar response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    find_measure(&value, metric).ok_or_else(|| format!("sonar response missing {metric} measure"))
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

    fn params(component: &str, server: &str, metric: &str) -> HashMap<String, String> {
        HashMap::from([
            ("component".to_string(), component.to_string()),
            ("server".to_string(), server.to_string()),
            ("metric".to_string(), metric.to_string()),
        ])
    }

    #[test]
    fn extracts_the_requested_metric_from_measures() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=code_smells",
            body: r#"{"component": {"measures": [{"metric": "code_smells", "value": "12"}]}}"#,
        };
        let value = resolve_generic(
            &params("comp", "https://sonarcloud.io", "code_smells"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "12");
    }

    #[test]
    fn appends_branch_to_the_query_when_provided() {
        let mut p = params("comp", "https://sonarcloud.io", "bugs");
        p.insert("branch".to_string(), "main".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=bugs&branch=main",
            body: r#"{"component": {"measures": [{"metric": "bugs", "value": "3"}]}}"#,
        };
        let value = resolve_generic(&p, &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn requires_component_server_and_metric_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_generic(&HashMap::new(), &Unused).is_err());
        assert!(resolve_generic(&params("", "https://sonarcloud.io", "bugs"), &Unused).is_err());
        assert!(resolve_generic(&params("comp", "", "bugs"), &Unused).is_err());
        assert!(resolve_generic(&params("comp", "https://sonarcloud.io", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_disallowed_param_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with invalid params")
            }
        }
        assert!(
            resolve_generic(&params("a?b=c", "https://sonarcloud.io", "bugs"), &Unused).is_err()
        );
        assert!(resolve_generic(&params("comp", "not-a-url", "bugs"), &Unused).is_err());
        assert!(
            resolve_generic(
                &params("comp", "https://sonarcloud.io", "bugs;drop"),
                &Unused
            )
            .is_err()
        );
    }

    #[test]
    fn errors_when_the_measure_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=bugs",
            body: r#"{"component": {"measures": []}}"#,
        };
        assert!(
            resolve_generic(&params("comp", "https://sonarcloud.io", "bugs"), &fetcher).is_err()
        );
    }
}
