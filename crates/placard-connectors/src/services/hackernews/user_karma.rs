use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_user_karma(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let id = params
        .get("id")
        .ok_or("hackernews-user-karma requires a data-id attribute")?;
    let id = validate_path_param("id", id)?;

    let url = format!("https://hacker-news.firebaseio.com/v0/user/{id}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "hackernews response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let karma = value
        .get("karma")
        .ok_or("hackernews response missing karma")?;
    karma
        .as_text()
        .ok_or_else(|| "karma was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://hacker-news.firebaseio.com/v0/user/pg.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(id: &str) -> HashMap<String, String> {
        HashMap::from([("id".to_string(), id.to_string())])
    }

    #[test]
    fn extracts_karma_from_a_hackernews_shaped_response() {
        let fetcher = FakeFetcher(r#"{"id": "pg", "karma": 157234, "created": 1160418092}"#);
        let value = resolve_user_karma(&params("pg"), &fetcher).unwrap();
        assert_eq!(value, "157234");
    }

    #[test]
    fn requires_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_user_karma(&HashMap::new(), &Unused).is_err());
        assert!(resolve_user_karma(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_user_karma(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_karma_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"id": "pg"}"#);
        assert!(resolve_user_karma(&params("pg"), &fetcher).is_err());
    }
}
