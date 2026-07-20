use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let scope = params
        .get("scope")
        .ok_or("jsr-version requires a data-scope attribute")?;
    let scope = scope.strip_prefix('@').unwrap_or(scope);
    let scope = validate_path_param("scope", scope)?;
    let package_name = params
        .get("package-name")
        .ok_or("jsr-version requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://jsr.io/@{scope}/{package_name}/meta.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "jsr response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let latest = value.get("latest").ok_or("jsr response missing latest")?;
    latest
        .as_text()
        .ok_or_else(|| "latest was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://jsr.io/@luca/flag/meta.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(scope: &str, package_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("scope".to_string(), scope.to_string()),
            ("package-name".to_string(), package_name.to_string()),
        ])
    }

    #[test]
    fn extracts_latest_from_a_jsr_shaped_response() {
        let fetcher = FakeFetcher(r#"{"latest": "1.2.3", "versions": {}}"#);
        let value = resolve_version(&params("luca", "flag"), &fetcher).unwrap();
        assert_eq!(value, "1.2.3");
    }

    #[test]
    fn strips_a_leading_at_sign_from_the_scope() {
        let fetcher = FakeFetcher(r#"{"latest": "1.2.3"}"#);
        let value = resolve_version(&params("@luca", "flag"), &fetcher).unwrap();
        assert_eq!(value, "1.2.3");
    }

    #[test]
    fn requires_scope_and_package_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("luca", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc", "flag"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_latest_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"versions": {}}"#);
        assert!(resolve_version(&params("luca", "flag"), &fetcher).is_err());
    }
}
