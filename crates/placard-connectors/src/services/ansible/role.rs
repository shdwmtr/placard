use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_role(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let namespace = params
        .get("namespace")
        .ok_or("ansible-role requires a data-namespace attribute")?;
    let name = params
        .get("name")
        .ok_or("ansible-role requires a data-name attribute")?;
    let namespace = validate_path_param("namespace", namespace)?;
    let name = validate_path_param("name", name)?;

    let url = format!(
        "https://galaxy.ansible.com/api/v1/roles/?namespace={namespace}&name={name}&limit=1"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "ansible galaxy response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let results = value
        .get("results")
        .ok_or("ansible galaxy response missing results")?;
    let Value::Array(results) = results else {
        return Err("ansible galaxy results was not a JSON array".to_string());
    };
    let first = results.first().ok_or("no ansible role found")?;
    first
        .get("download_count")
        .ok_or("ansible role entry missing download_count")?
        .as_text()
        .ok_or_else(|| "download_count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://galaxy.ansible.com/api/v1/roles/?namespace=openwisp&name=openwisp2&limit=1"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(namespace: &str, name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("namespace".to_string(), namespace.to_string()),
            ("name".to_string(), name.to_string()),
        ])
    }

    #[test]
    fn extracts_download_count_from_first_result() {
        let fetcher = FakeFetcher(r#"{"results": [{"download_count": 42}]}"#);
        let value = resolve_role(&params("openwisp", "openwisp2"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_namespace_and_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_role(&HashMap::new(), &Unused).is_err());
        assert!(resolve_role(&params("openwisp", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_role(&params("../etc", "openwisp2"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_results() {
        let fetcher = FakeFetcher(r#"{"results": []}"#);
        assert!(resolve_role(&params("openwisp", "openwisp2"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"results": [{"id": 1}]}"#);
        assert!(resolve_role(&params("openwisp", "openwisp2"), &fetcher).is_err());
    }
}
