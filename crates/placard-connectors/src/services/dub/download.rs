use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_download(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("dub-download requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;
    let interval = params
        .get("interval")
        .ok_or("dub-download requires a data-interval attribute")?;
    let field = match interval.as_str() {
        "dd" => "daily",
        "dw" => "weekly",
        "dm" => "monthly",
        "dt" => "total",
        other => {
            return Err(format!(
                "'interval' parameter '{other}' is not one of dd, dw, dm, dt"
            ));
        }
    };

    let mut url = format!("https://code.dlang.org/api/packages/{package}");
    if let Some(version) = params.get("version") {
        let version = validate_path_param("version", version)?;
        url.push('/');
        url.push_str(version);
    }
    url.push_str("/stats");

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "dub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let path = format!("downloads.{field}");
    let count = value
        .get(&path)
        .ok_or_else(|| format!("dub response missing {path}"))?;
    count
        .as_text()
        .ok_or_else(|| format!("{path} was not a plain value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    fn params(package: &str, interval: &str) -> HashMap<String, String> {
        HashMap::from([
            ("package".to_string(), package.to_string()),
            ("interval".to_string(), interval.to_string()),
        ])
    }

    #[test]
    fn extracts_total_downloads() {
        let fetcher = FakeFetcher(
            "https://code.dlang.org/api/packages/vibe-d/stats",
            r#"{"downloads": {"total": 123456, "monthly": 5000, "weekly": 1200, "daily": 200}}"#,
        );
        let value = resolve_download(&params("vibe-d", "dt"), &fetcher).unwrap();
        assert_eq!(value, "123456");
    }

    #[test]
    fn extracts_daily_downloads() {
        let fetcher = FakeFetcher(
            "https://code.dlang.org/api/packages/vibe-d/stats",
            r#"{"downloads": {"total": 123456, "monthly": 5000, "weekly": 1200, "daily": 200}}"#,
        );
        let value = resolve_download(&params("vibe-d", "dd"), &fetcher).unwrap();
        assert_eq!(value, "200");
    }

    #[test]
    fn uses_the_specific_version_endpoint_when_given() {
        let mut p = params("vibe-d", "dm");
        p.insert("version".to_string(), "0.8.4".to_string());
        let fetcher = FakeFetcher(
            "https://code.dlang.org/api/packages/vibe-d/0.8.4/stats",
            r#"{"downloads": {"total": 1, "monthly": 2, "weekly": 3, "daily": 4}}"#,
        );
        let value = resolve_download(&p, &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn requires_package_and_interval_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_download(&HashMap::new(), &Unused).is_err());
        assert!(resolve_download(&params("", "dt"), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_interval_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid interval")
            }
        }
        assert!(resolve_download(&params("vibe-d", "dy"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_download(&params("../etc", "dt"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(
            "https://code.dlang.org/api/packages/vibe-d/stats",
            r#"{"downloads": {}}"#,
        );
        assert!(resolve_download(&params("vibe-d", "dt"), &fetcher).is_err());
    }
}
