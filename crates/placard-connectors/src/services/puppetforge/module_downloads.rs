use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_module_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("puppetforge-module-downloads requires a data-user attribute")?;
    let module_name = params
        .get("module-name")
        .ok_or("puppetforge-module-downloads requires a data-module-name attribute")?;
    let user = validate_path_param("user", user)?;
    let module_name = validate_path_param("module-name", module_name)?;

    let url = format!("https://forgeapi.puppetlabs.com/v3/modules/{user}-{module_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "puppetforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let downloads = value
        .get("downloads")
        .ok_or("puppetforge response missing downloads")?;
    downloads
        .as_text()
        .ok_or_else(|| "downloads was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://forgeapi.puppetlabs.com/v3/modules/camptocamp-openldap"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, module_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("module-name".to_string(), module_name.to_string()),
        ])
    }

    #[test]
    fn extracts_downloads_from_a_puppetforge_module_response() {
        let fetcher = FakeFetcher(r#"{"downloads": 48213, "endorsement": null}"#);
        let value = resolve_module_downloads(&params("camptocamp", "openldap"), &fetcher).unwrap();
        assert_eq!(value, "48213");
    }

    #[test]
    fn requires_user_and_module_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_module_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_module_downloads(&params("camptocamp", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_module_downloads(&params("../etc", "openldap"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_downloads_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"endorsement": "approved"}"#);
        assert!(resolve_module_downloads(&params("camptocamp", "openldap"), &fetcher).is_err());
    }
}
