use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_hsts(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let domain = params
        .get("domain")
        .ok_or("hsts requires a data-domain attribute")?;
    let domain = validate_path_param("domain", domain)?;

    let url = format!("https://hstspreload.org/api/v2/status?domain={domain}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "hsts response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let status = value.get("status").ok_or("hsts response missing status")?;
    status
        .as_text()
        .ok_or_else(|| "status was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://hstspreload.org/api/v2/status?domain=github.com"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(domain: &str) -> HashMap<String, String> {
        HashMap::from([("domain".to_string(), domain.to_string())])
    }

    #[test]
    fn extracts_status_from_an_hsts_shaped_response() {
        let fetcher = FakeFetcher(r#"{"domain": "github.com", "status": "preloaded"}"#);
        let value = resolve_hsts(&params("github.com"), &fetcher).unwrap();
        assert_eq!(value, "preloaded");
    }

    #[test]
    fn requires_domain_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_hsts(&HashMap::new(), &Unused).is_err());
        assert!(resolve_hsts(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_hsts(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_status_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"domain": "github.com"}"#);
        assert!(resolve_hsts(&params("github.com"), &fetcher).is_err());
    }
}
