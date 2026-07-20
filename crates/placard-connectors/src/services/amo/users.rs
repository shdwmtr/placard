use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_users(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let addon_id = params
        .get("addon_id")
        .ok_or("amo-users requires a data-addon_id attribute")?;
    let addon_id = validate_path_param("addon_id", addon_id)?;

    let url = format!("https://addons.mozilla.org/api/v4/addons/addon/{addon_id}/");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "amo response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let users = value
        .get("average_daily_users")
        .ok_or("amo response missing average_daily_users")?;
    users
        .as_text()
        .ok_or_else(|| "average_daily_users was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://addons.mozilla.org/api/v4/addons/addon/dustman/"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(addon_id: &str) -> HashMap<String, String> {
        HashMap::from([("addon_id".to_string(), addon_id.to_string())])
    }

    #[test]
    fn extracts_average_daily_users_from_an_amo_shaped_response() {
        let fetcher = FakeFetcher(r#"{"average_daily_users": 3120}"#);
        let value = resolve_users(&params("dustman"), &fetcher).unwrap();
        assert_eq!(value, "3120");
    }

    #[test]
    fn requires_addon_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_users(&HashMap::new(), &Unused).is_err());
        assert!(resolve_users(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_users(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"weekly_downloads": 10}"#);
        assert!(resolve_users(&params("dustman"), &fetcher).is_err());
    }
}
