use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_taginfo(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let site = params
        .get("stackexchangesite")
        .ok_or("stackexchange-taginfo requires a data-stackexchangesite attribute")?;
    let query = params
        .get("query")
        .ok_or("stackexchange-taginfo requires a data-query attribute")?;
    let site = validate_path_param("stackexchangesite", site)?;
    let query = validate_path_param("query", query)?;

    let url = format!("https://api.stackexchange.com/2.2/tags/{query}/info?site={site}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "stackexchange response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let items = value
        .get("items")
        .ok_or("stackexchange response missing items")?;
    let Value::Array(items) = items else {
        return Err("stackexchange items field was not an array".to_string());
    };
    let first = items.first().ok_or("stackexchange response had no items")?;
    first
        .get("count")
        .ok_or("stackexchange item missing count")?
        .as_text()
        .ok_or_else(|| "count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.stackexchange.com/2.2/tags/gson/info?site=stackoverflow"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(site: &str, query: &str) -> HashMap<String, String> {
        HashMap::from([
            ("stackexchangesite".to_string(), site.to_string()),
            ("query".to_string(), query.to_string()),
        ])
    }

    #[test]
    fn extracts_count_from_the_first_item() {
        let fetcher = FakeFetcher(r#"{"items": [{"count": 3421}]}"#);
        let value = resolve_taginfo(&params("stackoverflow", "gson"), &fetcher).unwrap();
        assert_eq!(value, "3421");
    }

    #[test]
    fn requires_site_and_query_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_taginfo(&HashMap::new(), &Unused).is_err());
        assert!(resolve_taginfo(&params("stackoverflow", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_taginfo(&params("stackoverflow", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_items_is_empty() {
        let fetcher = FakeFetcher(r#"{"items": []}"#);
        assert!(resolve_taginfo(&params("stackoverflow", "gson"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_count_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"items": [{"name": "gson"}]}"#);
        assert!(resolve_taginfo(&params("stackoverflow", "gson"), &fetcher).is_err());
    }
}
