use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_user_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user_id = params
        .get("user-id")
        .ok_or("crates-user-downloads requires a data-user-id attribute")?;
    let user_id = validate_path_param("user-id", user_id)?;

    let url = format!("https://crates.io/api/v1/users/{user_id}/stats");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "crates.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let total = value
        .get("total_downloads")
        .ok_or("crates.io response missing total_downloads")?;
    total
        .as_text()
        .ok_or_else(|| "total_downloads was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://crates.io/api/v1/users/3027/stats");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user_id: &str) -> HashMap<String, String> {
        HashMap::from([("user-id".to_string(), user_id.to_string())])
    }

    #[test]
    fn extracts_total_downloads_from_a_user_stats_response() {
        let fetcher = FakeFetcher(r#"{"total_downloads": 481933}"#);
        let value = resolve_user_downloads(&params("3027"), &fetcher).unwrap();
        assert_eq!(value, "481933");
    }

    #[test]
    fn requires_user_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid user-id param")
            }
        }
        assert!(resolve_user_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_user_downloads(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_user_downloads(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"other": 1}"#);
        assert!(resolve_user_downloads(&params("3027"), &fetcher).is_err());
    }
}
