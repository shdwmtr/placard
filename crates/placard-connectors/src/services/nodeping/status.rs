use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

pub(crate) fn resolve_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let check_uuid = params
        .get("check-uuid")
        .ok_or("nodeping-status requires a data-check-uuid attribute")?;
    let check_uuid = validate_path_param("check-uuid", check_uuid)?;

    let url = format!("https://nodeping.com/reports/results/{check_uuid}/1?format=json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "nodeping response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let Value::Array(rows) = &value else {
        return Err("nodeping response was not an array".to_string());
    };
    let first = rows.first().ok_or("nodeping response had no result rows")?;
    let su = field(first, "su").ok_or("nodeping response missing su field")?;
    let is_up = matches!(su, Value::Bool(true));

    let message = if is_up {
        params
            .get("up_message")
            .cloned()
            .unwrap_or_else(|| "up".to_string())
    } else {
        params
            .get("down_message")
            .cloned()
            .unwrap_or_else(|| "down".to_string())
    };
    Ok(message)
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

    fn params(check_uuid: &str) -> HashMap<String, String> {
        HashMap::from([("check-uuid".to_string(), check_uuid.to_string())])
    }

    #[test]
    fn reports_up_when_su_is_true() {
        let fetcher = FakeFetcher {
            expected_url: "https://nodeping.com/reports/results/jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei/1?format=json",
            body: r#"[{"su": true}]"#,
        };
        let value =
            resolve_status(&params("jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei"), &fetcher).unwrap();
        assert_eq!(value, "up");
    }

    #[test]
    fn reports_down_when_su_is_false() {
        let fetcher = FakeFetcher {
            expected_url: "https://nodeping.com/reports/results/jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei/1?format=json",
            body: r#"[{"su": false}]"#,
        };
        let value =
            resolve_status(&params("jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei"), &fetcher).unwrap();
        assert_eq!(value, "down");
    }

    #[test]
    fn honors_custom_up_and_down_messages() {
        let fetcher = FakeFetcher {
            expected_url: "https://nodeping.com/reports/results/jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei/1?format=json",
            body: r#"[{"su": true}]"#,
        };
        let mut p = params("jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei");
        p.insert("up_message".to_string(), "online".to_string());
        let value = resolve_status(&p, &fetcher).unwrap();
        assert_eq!(value, "online");
    }

    #[test]
    fn requires_check_uuid_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid check-uuid")
            }
        }
        assert!(resolve_status(&HashMap::new(), &Unused).is_err());
        assert!(resolve_status(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_check_uuid_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid check-uuid")
            }
        }
        assert!(resolve_status(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_empty_or_malformed() {
        let fetcher = FakeFetcher {
            expected_url: "https://nodeping.com/reports/results/jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei/1?format=json",
            body: r#"[]"#,
        };
        assert!(resolve_status(&params("jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei"), &fetcher).is_err());
    }
}
