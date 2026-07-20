use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_package_for_downloads(package: &str) -> Result<(), String> {
    if package.is_empty() {
        return Err("'package' parameter must not be empty".to_string());
    }
    if package.starts_with('/')
        || package.ends_with('/')
        || package.contains("//")
        || package.contains("..")
    {
        return Err("'package' parameter is not a valid npm package name".to_string());
    }
    if !package.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' || c == '@'
    }) {
        return Err("'package' parameter contains disallowed characters".to_string());
    }
    Ok(())
}

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npm-downloads requires a data-package attribute")?;
    validate_package_for_downloads(package)?;
    let interval = params.get("interval").map(String::as_str).unwrap_or("dm");
    let query = match interval {
        "dw" => "point/last-week",
        "dm" => "point/last-month",
        "dy" => "point/last-year",
        "d18m" => "range/1000-01-01:3000-01-01",
        other => return Err(format!("unknown interval '{other}'")),
    };

    let url = format!("https://api.npmjs.org/downloads/{query}/{package}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "npm response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;

    if interval == "d18m" {
        let downloads = doc
            .get("downloads")
            .ok_or("npm response missing downloads")?;
        let Value::Array(items) = downloads else {
            return Err("npm response's downloads field was not an array".to_string());
        };
        let mut total = 0i64;
        for item in items {
            let count = item
                .get("downloads")
                .and_then(Value::as_text)
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or("npm response item missing downloads")?;
            total += count;
        }
        Ok(total.to_string())
    } else {
        let count = doc
            .get("downloads")
            .ok_or("npm response missing downloads")?;
        count
            .as_text()
            .ok_or_else(|| "downloads was not a plain value".to_string())
    }
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

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn defaults_to_last_month_point_downloads() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.npmjs.org/downloads/point/last-month/localeval",
            body: r#"{"downloads": 4213, "start": "2024-01-01", "end": "2024-01-31", "package": "localeval"}"#,
        };
        let value = resolve_downloads(&params("localeval"), &fetcher).unwrap();
        assert_eq!(value, "4213");
    }

    #[test]
    fn uses_the_week_endpoint_when_interval_is_dw() {
        let mut p = params("localeval");
        p.insert("interval".to_string(), "dw".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://api.npmjs.org/downloads/point/last-week/localeval",
            body: r#"{"downloads": 100}"#,
        };
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "100");
    }

    #[test]
    fn sums_the_18_month_range_endpoint() {
        let mut p = params("@angular/core");
        p.insert("interval".to_string(), "d18m".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://api.npmjs.org/downloads/range/1000-01-01:3000-01-01/@angular/core",
            body: r#"{"downloads": [{"downloads": 100, "day": "2024-01-01"}, {"downloads": 50, "day": "2024-01-02"}]}"#,
        };
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "150");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_intervals() {
        let mut p = params("localeval");
        p.insert("interval".to_string(), "decade".to_string());
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an unknown interval")
            }
        }
        assert!(resolve_downloads(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_downloads_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.npmjs.org/downloads/point/last-month/localeval",
            body: r#"{"package": "localeval"}"#,
        };
        assert!(resolve_downloads(&params("localeval"), &fetcher).is_err());
    }
}
