use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn interval_field(interval: &str) -> Result<&'static str, String> {
    match interval {
        "dd" => Ok("day"),
        "dw" => Ok("week"),
        "dt" => Ok("all"),
        _ => Err("'interval' parameter must be one of dd, dw, dt".to_string()),
    }
}

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let interval = params
        .get("interval")
        .ok_or("hexpm-downloads requires a data-interval attribute")?;
    let field = interval_field(interval)?;
    let package_name = params
        .get("package-name")
        .ok_or("hexpm-downloads requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://hex.pm/api/packages/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "hexpm response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let downloads = value
        .get(&format!("downloads.{field}"))
        .ok_or_else(|| format!("hexpm response missing downloads.{field}"))?;
    downloads
        .as_text()
        .ok_or_else(|| format!("downloads.{field} was not a plain value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://hex.pm/api/packages/plug");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(interval: &str, package_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("interval".to_string(), interval.to_string()),
            ("package-name".to_string(), package_name.to_string()),
        ])
    }

    #[test]
    fn extracts_the_field_matching_the_interval() {
        let fetcher = FakeFetcher(r#"{"downloads": {"all": 500000, "week": 1000, "day": 100}}"#);
        assert_eq!(
            resolve_downloads(&params("dd", "plug"), &fetcher).unwrap(),
            "100"
        );
        assert_eq!(
            resolve_downloads(&params("dw", "plug"), &fetcher).unwrap(),
            "1000"
        );
        assert_eq!(
            resolve_downloads(&params("dt", "plug"), &fetcher).unwrap(),
            "500000"
        );
    }

    #[test]
    fn rejects_an_unknown_interval() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid interval")
            }
        }
        assert!(resolve_downloads(&params("dm", "plug"), &Unused).is_err());
    }

    #[test]
    fn requires_interval_and_package_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("dd", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("dd", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"downloads": {}}"#);
        assert!(resolve_downloads(&params("dd", "plug"), &fetcher).is_err());
    }
}
