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

pub(crate) fn resolve_documented_api_density(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let component = params
        .get("component")
        .ok_or("sonar-documented-api-density requires a data-component attribute")?;
    let component = validate_component(component)?;
    let server = params
        .get("server")
        .ok_or("sonar-documented-api-density requires a data-server attribute")?;
    let server = validate_server(server)?;

    let mut url = format!(
        "{server}/api/measures/component?component={}&metricKeys=public_documented_api_density",
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
    let density = find_measure(&value, "public_documented_api_density")
        .ok_or("sonar response missing public_documented_api_density measure")?;
    Ok(format!("{density}%"))
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
    fn extracts_documented_api_density_from_measures() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=brave_brave-core&metricKeys=public_documented_api_density",
            body: r#"{"component": {"measures": [{"metric": "public_documented_api_density", "value": "85.7"}]}}"#,
        };
        let value = resolve_documented_api_density(
            &params("brave_brave-core", "https://sonarcloud.io"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "85.7%");
    }

    #[test]
    fn appends_branch_to_the_query_when_provided() {
        let mut p = params("brave_brave-core", "https://sonarcloud.io");
        p.insert("branch".to_string(), "main".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=brave_brave-core&metricKeys=public_documented_api_density&branch=main",
            body: r#"{"component": {"measures": [{"metric": "public_documented_api_density", "value": "90"}]}}"#,
        };
        let value = resolve_documented_api_density(&p, &fetcher).unwrap();
        assert_eq!(value, "90%");
    }

    #[test]
    fn requires_component_and_server_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_documented_api_density(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_documented_api_density(&params("", "https://sonarcloud.io"), &Unused).is_err()
        );
        assert!(resolve_documented_api_density(&params("comp", ""), &Unused).is_err());
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
            resolve_documented_api_density(&params("a?b=c", "https://sonarcloud.io"), &Unused)
                .is_err()
        );
        assert!(resolve_documented_api_density(&params("comp", "not-a-url"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_measure_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://sonarcloud.io/api/measures/component?component=comp&metricKeys=public_documented_api_density",
            body: r#"{"component": {"measures": []}}"#,
        };
        assert!(
            resolve_documented_api_density(&params("comp", "https://sonarcloud.io"), &fetcher)
                .is_err()
        );
    }
}
