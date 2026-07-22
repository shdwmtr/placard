use crate::Fetcher;
use std::collections::HashMap;

pub const PLACARDS_RENDERED_URL: &str = "https://placard.cc/placards-rendered";

pub(crate) fn resolve_placards_rendered(
    _params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let bytes = fetcher.fetch(PLACARDS_RENDERED_URL)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "placards-rendered response was not valid UTF-8".to_string())?;
    let trimmed = text.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err("placards-rendered response was not a plain integer".to_string());
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, PLACARDS_RENDERED_URL);
            Ok(self.0.as_bytes().to_vec())
        }
    }

    #[test]
    fn extracts_the_plain_integer_body() {
        let fetcher = FakeFetcher("142483");
        let value = resolve_placards_rendered(&HashMap::new(), &fetcher).unwrap();
        assert_eq!(value, "142483");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let fetcher = FakeFetcher("  142483\n");
        let value = resolve_placards_rendered(&HashMap::new(), &fetcher).unwrap();
        assert_eq!(value, "142483");
    }

    #[test]
    fn ignores_any_data_attributes_passed_by_mistake() {
        let fetcher = FakeFetcher("7");
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "shdwmtr".to_string());
        let value = resolve_placards_rendered(&params, &fetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn rejects_a_non_numeric_body() {
        struct FailFetcher;
        impl Fetcher for FailFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Ok(b"<html>not found</html>".to_vec())
            }
        }
        assert!(resolve_placards_rendered(&HashMap::new(), &FailFetcher).is_err());
    }

    #[test]
    fn rejects_an_empty_body() {
        struct EmptyFetcher;
        impl Fetcher for EmptyFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Ok(Vec::new())
            }
        }
        assert!(resolve_placards_rendered(&HashMap::new(), &EmptyFetcher).is_err());
    }

    #[test]
    fn propagates_fetch_errors() {
        struct FailingFetcher;
        impl Fetcher for FailingFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Err("connection refused".to_string())
            }
        }
        assert!(resolve_placards_rendered(&HashMap::new(), &FailingFetcher).is_err());
    }
}
