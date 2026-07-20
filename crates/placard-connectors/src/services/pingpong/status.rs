use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn render_status(status: &str) -> Result<&'static str, String> {
    match status {
        "Operational" => Ok("up"),
        "Major issues" => Ok("issues"),
        "Critical state" => Ok("down"),
        "Maintenance mode" => Ok("maintenance"),
        _ => Err("unknown status received".to_string()),
    }
}

pub(crate) fn resolve_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let api_key = params
        .get("api-key")
        .ok_or("pingpong-status requires a data-api-key attribute")?;
    let api_key = validate_path_param("api-key", api_key)?;

    let url = format!("https://api.pingpong.one/widget/shields/status/{api_key}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "pingpong response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let status = value
        .get("status")
        .and_then(|v| v.as_text())
        .ok_or("pingpong response missing status")?;

    render_status(&status).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.pingpong.one/widget/shields/status/sp_2e80bc00b6054faeb2b87e2464be337e"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(api_key: &str) -> HashMap<String, String> {
        HashMap::from([("api-key".to_string(), api_key.to_string())])
    }

    #[test]
    fn maps_operational_to_up() {
        let fetcher = FakeFetcher(r#"{"status": "Operational"}"#);
        let value =
            resolve_status(&params("sp_2e80bc00b6054faeb2b87e2464be337e"), &fetcher).unwrap();
        assert_eq!(value, "up");
    }

    #[test]
    fn maps_critical_state_to_down() {
        let fetcher = FakeFetcher(r#"{"status": "Critical state"}"#);
        let value =
            resolve_status(&params("sp_2e80bc00b6054faeb2b87e2464be337e"), &fetcher).unwrap();
        assert_eq!(value, "down");
    }

    #[test]
    fn requires_api_key_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid api key")
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
    fn errors_on_an_unknown_status() {
        let fetcher = FakeFetcher(r#"{"status": "Something else"}"#);
        assert!(resolve_status(&params("sp_2e80bc00b6054faeb2b87e2464be337e"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_status_field_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_status(&params("sp_2e80bc00b6054faeb2b87e2464be337e"), &fetcher).is_err());
    }
}
