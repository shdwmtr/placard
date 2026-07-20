use crate::Fetcher;
use crate::json::{self, Value};
use crate::services::validate_path_param;
use std::collections::HashMap;
use std::ops::Range;

fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => Some(*n as i64),
        _ => None,
    }
}

pub(crate) fn resolve_packagecontrol(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let interval = params
        .get("interval")
        .ok_or("packagecontrol requires a data-interval attribute")?;
    if !matches!(interval.as_str(), "dd" | "dw" | "dm" | "dt") {
        return Err(format!(
            "'interval' parameter '{interval}' must be one of dd, dw, dm, dt"
        ));
    }
    let package = params
        .get("package")
        .ok_or("packagecontrol requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://packagecontrol.io/packages/{package}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "packagecontrol response was not valid UTF-8".to_string())?;
    let root = json::parse(&text)?;

    if interval == "dt" {
        let total = root
            .get("installs.total")
            .ok_or("packagecontrol response missing installs.total")?;
        return total
            .as_text()
            .ok_or_else(|| "installs.total was not a plain value".to_string());
    }

    let data = root
        .get("installs.daily.data")
        .ok_or("packagecontrol response missing installs.daily.data")?;
    let Value::Array(platforms) = data else {
        return Err("packagecontrol installs.daily.data was not an array".to_string());
    };

    let day_range: Range<usize> = match interval.as_str() {
        "dd" => 1..2,
        "dw" => 0..7,
        "dm" => 0..30,
        _ => unreachable!(),
    };

    let mut sum: i64 = 0;
    for platform in platforms {
        let totals = match platform.get("totals") {
            Some(Value::Array(totals)) => totals,
            _ => return Err("packagecontrol platform entry missing totals array".to_string()),
        };
        for idx in day_range.clone() {
            let n = totals
                .get(idx)
                .and_then(as_i64)
                .ok_or("packagecontrol totals entry was not numeric")?;
            sum += n;
        }
    }
    Ok(sum.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://packagecontrol.io/packages/GitGutter.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(interval: &str, package: &str) -> HashMap<String, String> {
        HashMap::from([
            ("interval".to_string(), interval.to_string()),
            ("package".to_string(), package.to_string()),
        ])
    }

    fn body() -> &'static str {
        r#"{
            "installs": {
                "total": 999,
                "daily": {
                    "data": [
                        {"totals": [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30]},
                        {"totals": [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]}
                    ]
                }
            }
        }"#
    }

    #[test]
    fn extracts_the_total_downloads() {
        let fetcher = FakeFetcher(body());
        let value = resolve_packagecontrol(&params("dt", "GitGutter"), &fetcher).unwrap();
        assert_eq!(value, "999");
    }

    #[test]
    fn sums_yesterdays_downloads_across_platforms_for_daily() {
        let fetcher = FakeFetcher(body());
        let value = resolve_packagecontrol(&params("dd", "GitGutter"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn sums_the_first_seven_days_across_platforms_for_weekly() {
        let fetcher = FakeFetcher(body());
        let value = resolve_packagecontrol(&params("dw", "GitGutter"), &fetcher).unwrap();
        let expected: i64 = (1..=7).sum::<i64>() + 7;
        assert_eq!(value, expected.to_string());
    }

    #[test]
    fn sums_the_first_thirty_days_across_platforms_for_monthly() {
        let fetcher = FakeFetcher(body());
        let value = resolve_packagecontrol(&params("dm", "GitGutter"), &fetcher).unwrap();
        let expected: i64 = (1..=30).sum::<i64>() + 30;
        assert_eq!(value, expected.to_string());
    }

    #[test]
    fn requires_interval_and_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_packagecontrol(&HashMap::new(), &Unused).is_err());
        assert!(resolve_packagecontrol(&params("bogus", "GitGutter"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_packagecontrol(&params("dt", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"installs": {"total": 1}}"#);
        assert!(resolve_packagecontrol(&params("dd", "GitGutter"), &fetcher).is_err());
    }
}
