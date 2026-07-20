use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_dependents(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let crate_name = params
        .get("crate")
        .ok_or("crates-dependents requires a data-crate attribute")?;
    let crate_name = validate_path_param("crate", crate_name)?;

    let url = format!("https://crates.io/api/v1/crates/{crate_name}/reverse_dependencies");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "crates.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let total = value
        .get("meta.total")
        .ok_or("crates.io response missing meta.total")?;
    total
        .as_text()
        .ok_or_else(|| "meta.total was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://crates.io/api/v1/crates/tokio/reverse_dependencies"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(crate_name: &str) -> HashMap<String, String> {
        HashMap::from([("crate".to_string(), crate_name.to_string())])
    }

    #[test]
    fn extracts_dependent_count_from_a_crates_shaped_response() {
        let fetcher = FakeFetcher(r#"{"dependencies": [], "meta": {"total": 731}}"#);
        let value = resolve_dependents(&params("tokio"), &fetcher).unwrap();
        assert_eq!(value, "731");
    }

    #[test]
    fn requires_crate_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid crate param")
            }
        }
        assert!(resolve_dependents(&HashMap::new(), &Unused).is_err());
        assert!(resolve_dependents(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_dependents(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"dependencies": []}"#);
        assert!(resolve_dependents(&params("tokio"), &fetcher).is_err());
    }
}
