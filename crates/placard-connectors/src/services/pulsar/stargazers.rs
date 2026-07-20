use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_stargazers(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("pulsar-stargazers requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://api.pulsar-edit.dev/api/packages/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "pulsar response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("stargazers_count")
        .ok_or("pulsar response missing stargazers_count")?;
    count
        .as_text()
        .ok_or_else(|| "stargazers_count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.pulsar-edit.dev/api/packages/hey-pane");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_the_stargazer_count() {
        let fetcher = FakeFetcher(r#"{"name": "hey-pane", "stargazers_count": 57}"#);
        let value = resolve_stargazers(&params("hey-pane"), &fetcher).unwrap();
        assert_eq!(value, "57");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package name")
            }
        }
        assert!(resolve_stargazers(&HashMap::new(), &Unused).is_err());
        assert!(resolve_stargazers(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_stargazers(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_stargazers_count_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "hey-pane"}"#);
        assert!(resolve_stargazers(&params("hey-pane"), &fetcher).is_err());
    }
}
