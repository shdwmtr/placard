use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let addon_id = params
        .get("addon_id")
        .ok_or("amo-downloads requires a data-addon_id attribute")?;
    let addon_id = validate_path_param("addon_id", addon_id)?;

    let url = format!("https://addons.mozilla.org/api/v4/addons/addon/{addon_id}/");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "amo response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let downloads = value
        .get("weekly_downloads")
        .ok_or("amo response missing weekly_downloads")?;
    downloads
        .as_text()
        .ok_or_else(|| "weekly_downloads was not a plain value".to_string())
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
    fn extracts_weekly_downloads_from_an_amo_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"weekly_downloads": 4821, "average_daily_users": 100, "current_version": {"version": "1.0"}, "ratings": {"average": 4.5}}"#,
        );
        let value = resolve_downloads(&params("dustman"), &fetcher).unwrap();
        assert_eq!(value, "4821");
    }

    #[test]
    fn requires_addon_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"average_daily_users": 100}"#);
        assert!(resolve_downloads(&params("dustman"), &fetcher).is_err());
    }
}
