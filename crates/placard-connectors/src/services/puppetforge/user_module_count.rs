use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_user_module_count(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("puppetforge-user-module-count requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;

    let url = format!("https://forgeapi.puppetlabs.com/v3/users/{user}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "puppetforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("module_count")
        .ok_or("puppetforge response missing module_count")?;
    count
        .as_text()
        .ok_or_else(|| "module_count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://forgeapi.puppetlabs.com/v3/users/camptocamp");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str) -> HashMap<String, String> {
        HashMap::from([("user".to_string(), user.to_string())])
    }

    #[test]
    fn extracts_module_count_from_a_puppetforge_user_response() {
        let fetcher = FakeFetcher(r#"{"module_count": 42, "release_count": 210}"#);
        let value = resolve_user_module_count(&params("camptocamp"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_user_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_user_module_count(&HashMap::new(), &Unused).is_err());
        assert!(resolve_user_module_count(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_user_module_count(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_module_count_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"release_count": 210}"#);
        assert!(resolve_user_module_count(&params("camptocamp"), &fetcher).is_err());
    }
}
