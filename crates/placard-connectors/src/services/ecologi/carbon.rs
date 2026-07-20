use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_carbon(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let username = params
        .get("username")
        .ok_or("ecologi-carbon requires a data-username attribute")?;
    let username = validate_path_param("username", username)?;

    let url = format!("https://public.ecologi.com/users/{username}/carbon-offset");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "ecologi response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let total = value.get("total").ok_or("ecologi response missing total")?;
    total
        .as_text()
        .ok_or_else(|| "total was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://public.ecologi.com/users/ecologi/carbon-offset"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(username: &str) -> HashMap<String, String> {
        HashMap::from([("username".to_string(), username.to_string())])
    }

    #[test]
    fn extracts_total_from_an_ecologi_shaped_response() {
        let fetcher = FakeFetcher(r#"{"total": 12.5}"#);
        let value = resolve_carbon(&params("ecologi"), &fetcher).unwrap();
        assert_eq!(value, "12.5");
    }

    #[test]
    fn requires_username_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_carbon(&HashMap::new(), &Unused).is_err());
        assert!(resolve_carbon(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_carbon(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"other": 1}"#);
        assert!(resolve_carbon(&params("ecologi"), &fetcher).is_err());
    }
}
