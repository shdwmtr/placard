use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_rank(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let period = params
        .get("period")
        .ok_or("gem-rank requires a data-period attribute")?;
    if !matches!(period.as_str(), "rt" | "rd") {
        return Err("'period' parameter must be one of rt, rd".to_string());
    }
    let gem = params
        .get("gem")
        .ok_or("gem-rank requires a data-gem attribute")?;
    let gem = validate_path_param("gem", gem)?;

    let (endpoint, field) = if period == "rt" {
        ("total_ranking.json", "total_ranking")
    } else {
        ("daily_ranking.json", "daily_ranking")
    };

    let url = format!("http://bestgems.org/api/v1/gems/{gem}/{endpoint}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "bestgems response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(entries) = value else {
        return Err("bestgems response was not an array".to_string());
    };
    let first = entries.first().ok_or("bestgems response was empty")?;
    first
        .get(field)
        .and_then(|v| v.as_text())
        .ok_or_else(|| format!("bestgems response missing {field}"))
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

    fn params(period: &str, gem: &str) -> HashMap<String, String> {
        HashMap::from([
            ("period".to_string(), period.to_string()),
            ("gem".to_string(), gem.to_string()),
        ])
    }

    #[test]
    fn extracts_total_ranking() {
        let fetcher = FakeFetcher {
            expected_url: "http://bestgems.org/api/v1/gems/puppet/total_ranking.json",
            body: r#"[{"total_ranking": 42}]"#,
        };
        let value = resolve_rank(&params("rt", "puppet"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn extracts_daily_ranking() {
        let fetcher = FakeFetcher {
            expected_url: "http://bestgems.org/api/v1/gems/puppet/daily_ranking.json",
            body: r#"[{"daily_ranking": 7}]"#,
        };
        let value = resolve_rank(&params("rd", "puppet"), &fetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn requires_period_and_gem_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_rank(&HashMap::new(), &Unused).is_err());
        assert!(resolve_rank(&params("rt", ""), &Unused).is_err());
        assert!(resolve_rank(&params("bogus", "puppet"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_rank(&params("rt", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "http://bestgems.org/api/v1/gems/puppet/total_ranking.json",
            body: r#"[{"total_ranking": null}]"#,
        };
        assert!(resolve_rank(&params("rt", "puppet"), &fetcher).is_err());
    }
}
