use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_cookbook(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let cookbook = params
        .get("cookbook")
        .ok_or("cookbook requires a data-cookbook attribute")?;
    let cookbook = validate_path_param("cookbook", cookbook)?;

    let url = format!("https://supermarket.chef.io/api/v1/cookbooks/{cookbook}/versions/latest");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "cookbook response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("cookbook response missing version")?;
    version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: &'static str,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(cookbook: &str) -> HashMap<String, String> {
        HashMap::from([("cookbook".to_string(), cookbook.to_string())])
    }

    #[test]
    fn extracts_the_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://supermarket.chef.io/api/v1/cookbooks/chef-sugar/versions/latest",
            body: r#"{"version": "5.1.5"}"#,
        };
        let value = resolve_cookbook(&params("chef-sugar"), &fetcher).unwrap();
        assert_eq!(value, "5.1.5");
    }

    #[test]
    fn requires_cookbook_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid cookbook")
            }
        }
        assert!(resolve_cookbook(&HashMap::new(), &Unused).is_err());
        assert!(resolve_cookbook(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid cookbook")
            }
        }
        assert!(resolve_cookbook(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://supermarket.chef.io/api/v1/cookbooks/chef-sugar/versions/latest",
            body: r#"{"other": 1}"#,
        };
        assert!(resolve_cookbook(&params("chef-sugar"), &fetcher).is_err());
    }
}
