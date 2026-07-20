use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-downloads requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;
    let period = params
        .get("period")
        .ok_or("pypi-downloads requires a data-period attribute")?;
    let field = match period.as_str() {
        "dd" => "last_day",
        "dw" => "last_week",
        "dm" => "last_month",
        other => {
            return Err(format!(
                "'period' parameter '{other}' is not one of dd, dw, dm"
            ));
        }
    };

    let url = format!(
        "https://pypistats.org/api/packages/{}/recent",
        package.to_lowercase()
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "pypistats response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let path = format!("data.{field}");
    let count = value
        .get(&path)
        .ok_or_else(|| format!("pypistats response missing {path}"))?;
    count
        .as_text()
        .ok_or_else(|| format!("{path} was not a plain value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://pypistats.org/api/packages/requests/recent");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str, period: &str) -> HashMap<String, String> {
        HashMap::from([
            ("package".to_string(), package.to_string()),
            ("period".to_string(), period.to_string()),
        ])
    }

    #[test]
    fn extracts_last_day_downloads() {
        let fetcher = FakeFetcher(
            r#"{"data": {"last_day": 12345, "last_week": 90000, "last_month": 400000}}"#,
        );
        let value = resolve_downloads(&params("requests", "dd"), &fetcher).unwrap();
        assert_eq!(value, "12345");
    }

    #[test]
    fn extracts_last_month_downloads() {
        let fetcher = FakeFetcher(
            r#"{"data": {"last_day": 12345, "last_week": 90000, "last_month": 400000}}"#,
        );
        let value = resolve_downloads(&params("requests", "dm"), &fetcher).unwrap();
        assert_eq!(value, "400000");
    }

    #[test]
    fn lowercases_package_name_in_the_url() {
        struct FakeFetcher2;
        impl Fetcher for FakeFetcher2 {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(url, "https://pypistats.org/api/packages/requests/recent");
                Ok(r#"{"data": {"last_week": 1}}"#.as_bytes().to_vec())
            }
        }
        let value = resolve_downloads(&params("Requests", "dw"), &FakeFetcher2).unwrap();
        assert_eq!(value, "1");
    }

    #[test]
    fn requires_package_and_period_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("", "dd"), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_period_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid period")
            }
        }
        assert!(resolve_downloads(&params("requests", "dy"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc", "dd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"data": {}}"#);
        assert!(resolve_downloads(&params("requests", "dd"), &fetcher).is_err());
    }
}
