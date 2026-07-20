use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_server(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("'server' parameter must not be empty".to_string());
    }
    let trimmed = value.trim_end_matches('/');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
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

fn validate_build_id(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'build-id' parameter must not be empty".to_string());
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("'build-id' parameter contains disallowed characters".to_string());
    }
    Ok(value)
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn find_coverage(value: &Value) -> Option<f64> {
    let Value::Array(items) = value.get("property")? else {
        return None;
    };
    let mut covered: Option<f64> = None;
    let mut total: Option<f64> = None;
    for item in items {
        let name = item.get("name").and_then(|v| v.as_text());
        let raw = item.get("value").and_then(|v| v.as_text());
        match (name.as_deref(), raw) {
            (Some("CodeCoverageAbsSCovered"), Some(v)) => covered = v.parse::<f64>().ok(),
            (Some("CodeCoverageAbsSTotal"), Some(v)) => total = v.parse::<f64>().ok(),
            _ => {}
        }
        if let (Some(covered), Some(total)) = (covered, total) {
            return Some(if covered == 0.0 {
                0.0
            } else {
                (covered / total) * 100.0
            });
        }
    }
    None
}

/// TeamCity's REST API requires authentication for most build configs, but
/// falls back to a `?guest=1` anonymous-access mode when no credentials are
/// supplied, which is the only mode this connector uses.
pub(crate) fn resolve_coverage(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let build_id = params
        .get("build-id")
        .ok_or("teamcity-coverage requires a data-build-id attribute")?;
    let build_id = validate_build_id(build_id)?;
    let server = match params.get("server") {
        Some(s) => validate_server(s)?,
        None => "https://teamcity.jetbrains.com".to_string(),
    };

    let build_locator = format!("buildType:(id:{build_id})");
    let url = format!(
        "{server}/app/rest/builds/{}/statistics?guest=1",
        percent_encode(&build_locator)
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "teamcity response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let coverage = find_coverage(&value).ok_or("teamcity response missing coverage statistics")?;
    Ok(format!("{}%", coverage.round() as i64))
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

    fn params(build_id: &str) -> HashMap<String, String> {
        HashMap::from([("build-id".to_string(), build_id.to_string())])
    }

    #[test]
    fn computes_coverage_percentage_from_covered_and_total() {
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3AFileHelpersStable)/statistics?guest=1",
            body: r#"{"property": [{"name": "CodeCoverageAbsSCovered", "value": "750"}, {"name": "CodeCoverageAbsSTotal", "value": "1000"}]}"#,
        };
        let value = resolve_coverage(&params("FileHelpersStable"), &fetcher).unwrap();
        assert_eq!(value, "75%");
    }

    #[test]
    fn reports_zero_when_covered_is_zero() {
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3AFileHelpersStable)/statistics?guest=1",
            body: r#"{"property": [{"name": "CodeCoverageAbsSCovered", "value": "0"}, {"name": "CodeCoverageAbsSTotal", "value": "1000"}]}"#,
        };
        let value = resolve_coverage(&params("FileHelpersStable"), &fetcher).unwrap();
        assert_eq!(value, "0%");
    }

    #[test]
    fn ignores_field_order_and_extra_properties() {
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3AFileHelpersStable)/statistics?guest=1",
            body: r#"{"property": [{"name": "BuildDuration", "value": "12345"}, {"name": "CodeCoverageAbsSTotal", "value": "200"}, {"name": "CodeCoverageAbsSCovered", "value": "50"}]}"#,
        };
        let value = resolve_coverage(&params("FileHelpersStable"), &fetcher).unwrap();
        assert_eq!(value, "25%");
    }

    #[test]
    fn uses_a_custom_server_when_provided() {
        let mut p = params("FileHelpersStable");
        p.insert("server".to_string(), "https://ci.example.com/".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://ci.example.com/app/rest/builds/buildType%3A(id%3AFileHelpersStable)/statistics?guest=1",
            body: r#"{"property": [{"name": "CodeCoverageAbsSCovered", "value": "1"}, {"name": "CodeCoverageAbsSTotal", "value": "2"}]}"#,
        };
        let value = resolve_coverage(&p, &fetcher).unwrap();
        assert_eq!(value, "50%");
    }

    #[test]
    fn requires_build_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid build-id")
            }
        }
        assert!(resolve_coverage(&HashMap::new(), &Unused).is_err());
        assert!(resolve_coverage(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_build_id() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid build-id")
            }
        }
        assert!(resolve_coverage(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_coverage_data_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3AFileHelpersStable)/statistics?guest=1",
            body: r#"{"property": [{"name": "BuildDuration", "value": "12345"}]}"#,
        };
        assert!(resolve_coverage(&params("FileHelpersStable"), &fetcher).is_err());
    }
}
