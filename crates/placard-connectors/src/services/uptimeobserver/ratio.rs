use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn format_ratio(n: f64) -> String {
    let rounded = (n * 1000.0).round() / 1000.0;
    let mut s = format!("{rounded:.3}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

pub(crate) fn resolve_ratio(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let monitor_key = params
        .get("monitor-key")
        .ok_or("uptimeobserver-ratio requires a data-monitor-key attribute")?;
    let monitor_key = validate_path_param("monitor-key", monitor_key)?;

    let field = match params.get("period").map(String::as_str) {
        Some("1") => "uptime24h",
        Some("7") => "uptime7d",
        _ => "uptime30d",
    };

    let url = format!("https://app.uptimeobserver.com/api/monitor/status/{monitor_key}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "uptimeobserver response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    if let Some(error) = value.get("error").and_then(Value::as_text) {
        return Err(error);
    }

    let ratio = value
        .get(field)
        .ok_or_else(|| format!("uptimeobserver response missing {field}"))?;
    match ratio {
        Value::Number(n) => Ok(format!("{}%", format_ratio(*n))),
        _ => Err(format!("{field} was not a number")),
    }
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

    fn params(monitor_key: &str) -> HashMap<String, String> {
        HashMap::from([("monitor-key".to_string(), monitor_key.to_string())])
    }

    #[test]
    fn extracts_the_30_day_ratio_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"status": "UP", "uptime24h": 100, "uptime7d": 99.5, "uptime30d": 99.123}"#,
        };
        let value = resolve_ratio(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).unwrap();
        assert_eq!(value, "99.123%");
    }

    #[test]
    fn honors_the_period_param() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"status": "UP", "uptime24h": 100, "uptime7d": 99.5, "uptime30d": 99.123}"#,
        };
        let mut p = params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41");
        p.insert("period".to_string(), "7".to_string());
        let value = resolve_ratio(&p, &fetcher).unwrap();
        assert_eq!(value, "99.5%");
    }

    #[test]
    fn strips_trailing_zeros() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"status": "UP", "uptime24h": 100, "uptime7d": 99.5, "uptime30d": 100}"#,
        };
        let value = resolve_ratio(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).unwrap();
        assert_eq!(value, "100%");
    }

    #[test]
    fn requires_monitor_key_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_ratio(&HashMap::new(), &Unused).is_err());
        assert!(resolve_ratio(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_ratio(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn propagates_the_upstream_error_message() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"error": "api_key not found."}"#,
        };
        let err =
            resolve_ratio(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).unwrap_err();
        assert_eq!(err, "api_key not found.");
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"status": "UP"}"#,
        };
        assert!(resolve_ratio(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).is_err());
    }
}
