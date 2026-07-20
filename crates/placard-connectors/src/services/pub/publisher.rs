use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_publisher(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pub-publisher requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://pub.dev/api/packages/{package}/publisher");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "pub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    match value.get("publisherId") {
        None | Some(Value::Null) => Ok("unverified".to_string()),
        Some(publisher_id) => publisher_id
            .as_text()
            .ok_or_else(|| "publisherId was not a plain value".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://pub.dev/api/packages/analysis_options/publisher"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_the_publisher_id() {
        let fetcher = FakeFetcher(r#"{"publisherId": "dart.dev"}"#);
        let value = resolve_publisher(&params("analysis_options"), &fetcher).unwrap();
        assert_eq!(value, "dart.dev");
    }

    #[test]
    fn reports_unverified_when_publisher_id_is_null() {
        let fetcher = FakeFetcher(r#"{"publisherId": null}"#);
        let value = resolve_publisher(&params("analysis_options"), &fetcher).unwrap();
        assert_eq!(value, "unverified");
    }

    #[test]
    fn reports_unverified_when_publisher_id_key_is_absent() {
        let fetcher = FakeFetcher(r#"{}"#);
        let value = resolve_publisher(&params("analysis_options"), &fetcher).unwrap();
        assert_eq!(value, "unverified");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_publisher(&HashMap::new(), &Unused).is_err());
        assert!(resolve_publisher(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_publisher(&params("../etc"), &Unused).is_err());
    }
}
