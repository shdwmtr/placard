use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_owner(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("gem-owner requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;

    let url = format!("https://rubygems.org/api/v1/owners/{user}/gems.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "rubygems response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(gems) = value else {
        return Err("rubygems response was not an array".to_string());
    };
    Ok(gems.len().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://rubygems.org/api/v1/owners/raphink/gems.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str) -> HashMap<String, String> {
        HashMap::from([("user".to_string(), user.to_string())])
    }

    #[test]
    fn counts_the_gems_owned_by_the_user() {
        let fetcher = FakeFetcher(r#"[{"name": "puppet"}, {"name": "facter"}, {"name": "hiera"}]"#);
        let value = resolve_owner(&params("raphink"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn requires_user_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_owner(&HashMap::new(), &Unused).is_err());
        assert!(resolve_owner(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_owner(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_response_is_not_an_array() {
        let fetcher = FakeFetcher(r#"{"error": "not found"}"#);
        assert!(resolve_owner(&params("raphink"), &fetcher).is_err());
    }
}
