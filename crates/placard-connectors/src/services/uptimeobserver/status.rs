use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let monitor_key = params
        .get("monitor-key")
        .ok_or("uptimeobserver-status requires a data-monitor-key attribute")?;
    let monitor_key = validate_path_param("monitor-key", monitor_key)?;

    let up_message = params.get("up_message").map(String::as_str).unwrap_or("up");
    let down_message = params
        .get("down_message")
        .map(String::as_str)
        .unwrap_or("down");

    let url = format!("https://app.uptimeobserver.com/api/monitor/status/{monitor_key}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "uptimeobserver response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    if let Some(error) = value.get("error").and_then(Value::as_text) {
        return Err(error);
    }

    let status = value
        .get("status")
        .ok_or("uptimeobserver response missing status")?;
    let status_text = status
        .as_text()
        .ok_or_else(|| "status was not a plain value".to_string())?;

    Ok(match status_text.to_ascii_uppercase().as_str() {
        "UP" => up_message.to_string(),
        "DOWN" => down_message.to_string(),
        "PAUSED" => "paused".to_string(),
        _ => status_text.to_ascii_lowercase(),
    })
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
    fn reports_up_for_an_up_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"status": "UP", "uptime24h": 100, "uptime7d": 100, "uptime30d": 100}"#,
        };
        let value =
            resolve_status(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).unwrap();
        assert_eq!(value, "up");
    }

    #[test]
    fn reports_paused_for_a_paused_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"status": "PAUSED", "uptime24h": 100, "uptime7d": 100, "uptime30d": 100}"#,
        };
        let value =
            resolve_status(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).unwrap();
        assert_eq!(value, "paused");
    }

    #[test]
    fn honors_custom_up_and_down_messages() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"status": "DOWN", "uptime24h": 0, "uptime7d": 0, "uptime30d": 0}"#,
        };
        let mut p = params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41");
        p.insert("down_message".to_string(), "offline".to_string());
        let value = resolve_status(&p, &fetcher).unwrap();
        assert_eq!(value, "offline");
    }

    #[test]
    fn requires_monitor_key_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_status(&HashMap::new(), &Unused).is_err());
        assert!(resolve_status(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_status(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn propagates_the_upstream_error_message() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"error": "api_key not found."}"#,
        };
        let err =
            resolve_status(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).unwrap_err();
        assert_eq!(err, "api_key not found.");
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://app.uptimeobserver.com/api/monitor/status/33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            body: r#"{"uptime24h": 100}"#,
        };
        assert!(resolve_status(&params("33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41"), &fetcher).is_err());
    }
}
