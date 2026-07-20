use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_uptime(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let api_key = params
        .get("api-key")
        .ok_or("pingpong-uptime requires a data-api-key attribute")?;
    let api_key = validate_path_param("api-key", api_key)?;

    let url = format!("https://api.pingpong.one/widget/shields/uptime/{api_key}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "pingpong response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let uptime = value
        .get("uptime")
        .and_then(|v| v.as_text())
        .ok_or("pingpong response missing uptime")?;

    Ok(format!("{uptime}%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.pingpong.one/widget/shields/uptime/sp_2e80bc00b6054faeb2b87e2464be337e"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(api_key: &str) -> HashMap<String, String> {
        HashMap::from([("api-key".to_string(), api_key.to_string())])
    }

    #[test]
    fn extracts_the_uptime_percentage() {
        let fetcher = FakeFetcher(r#"{"uptime": 99.95}"#);
        let value =
            resolve_uptime(&params("sp_2e80bc00b6054faeb2b87e2464be337e"), &fetcher).unwrap();
        assert_eq!(value, "99.95%");
    }

    #[test]
    fn formats_a_whole_number_uptime_without_a_decimal_point() {
        let fetcher = FakeFetcher(r#"{"uptime": 100}"#);
        let value =
            resolve_uptime(&params("sp_2e80bc00b6054faeb2b87e2464be337e"), &fetcher).unwrap();
        assert_eq!(value, "100%");
    }

    #[test]
    fn requires_api_key_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid api key")
            }
        }
        assert!(resolve_uptime(&HashMap::new(), &Unused).is_err());
        assert!(resolve_uptime(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_uptime(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_uptime_field_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_uptime(&params("sp_2e80bc00b6054faeb2b87e2464be337e"), &fetcher).is_err());
    }
}
