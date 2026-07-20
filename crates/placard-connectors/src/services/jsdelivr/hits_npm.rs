use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn period_word(period: &str) -> Result<&'static str, String> {
    match period {
        "hd" => Ok("day"),
        "hw" => Ok("week"),
        "hm" => Ok("month"),
        "hy" => Ok("year"),
        _ => Err("'period' parameter must be one of hd, hw, hm, hy".to_string()),
    }
}

pub(crate) fn resolve_hits_npm(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let period = params
        .get("period")
        .ok_or("jsdelivr-hits-npm requires a data-period attribute")?;
    let period_word = period_word(period)?;
    let package = params
        .get("package")
        .ok_or("jsdelivr-hits-npm requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let full_package = match params.get("scope") {
        Some(scope) if !scope.is_empty() => {
            let stripped = scope.strip_prefix('@').unwrap_or(scope);
            let scope = validate_path_param("scope", stripped)?;
            format!("@{scope}/{package}")
        }
        _ => package.to_string(),
    };

    let url =
        format!("https://data.jsdelivr.com/v1/package/npm/{full_package}/stats/date/{period_word}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "jsdelivr response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let total = value
        .get("total")
        .ok_or("jsdelivr response missing total")?;
    total
        .as_text()
        .ok_or_else(|| "total was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.1);
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(period: &str, package: &str, scope: Option<&str>) -> HashMap<String, String> {
        let mut map = HashMap::from([
            ("period".to_string(), period.to_string()),
            ("package".to_string(), package.to_string()),
        ]);
        if let Some(scope) = scope {
            map.insert("scope".to_string(), scope.to_string());
        }
        map
    }

    #[test]
    fn extracts_total_from_a_jsdelivr_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"total": 42}"#,
            "https://data.jsdelivr.com/v1/package/npm/fire/stats/date/month",
        );
        let value = resolve_hits_npm(&params("hm", "fire", None), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn includes_a_scope_when_provided() {
        let fetcher = FakeFetcher(
            r#"{"total": 7}"#,
            "https://data.jsdelivr.com/v1/package/npm/@angular/fire/stats/date/day",
        );
        let value = resolve_hits_npm(&params("hd", "fire", Some("@angular")), &fetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn requires_period_and_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_hits_npm(&HashMap::new(), &Unused).is_err());
        assert!(resolve_hits_npm(&params("hm", "", None), &Unused).is_err());
    }

    #[test]
    fn rejects_an_invalid_period() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid period")
            }
        }
        assert!(resolve_hits_npm(&params("bogus", "fire", None), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_hits_npm(&params("hm", "../etc", None), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(
            r#"{"day": "2024-01-01"}"#,
            "https://data.jsdelivr.com/v1/package/npm/fire/stats/date/month",
        );
        assert!(resolve_hits_npm(&params("hm", "fire", None), &fetcher).is_err());
    }
}
