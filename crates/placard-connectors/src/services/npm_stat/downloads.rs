use super::validate_path_param;
use crate::Fetcher;
use crate::json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Converts a day count since the Unix epoch (1970-01-01) into `(year,
/// month, day)`, via Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn format_ymd(y: i64, m: u32, d: u32) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

/// Subtracts one calendar week/month/year from `today_days`, formatted as
/// `YYYY-MM-DD`, clamping the day-of-month the same way `dayjs().subtract()`
/// would (e.g. March 31st minus one month becomes February 28th/29th).
fn one_unit_before(today_days: i64, interval: &str) -> Result<String, String> {
    let (y, m, d) = civil_from_days(today_days);
    match interval {
        "dw" => {
            let (y2, m2, d2) = civil_from_days(today_days - 7);
            Ok(format_ymd(y2, m2, d2))
        }
        "dm" => {
            let (y2, m2) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
            let d2 = d.min(days_in_month(y2, m2));
            Ok(format_ymd(y2, m2, d2))
        }
        "dy" => {
            let y2 = y - 1;
            let d2 = d.min(days_in_month(y2, m));
            Ok(format_ymd(y2, m, d2))
        }
        other => Err(format!("unknown interval '{other}'")),
    }
}

fn total_downloads(doc: &Value) -> Result<i64, String> {
    let Value::Object(packages) = doc else {
        return Err("npm-stat response was not an object".to_string());
    };
    let mut total = 0i64;
    for (_, package_downloads) in packages {
        let Value::Object(days) = package_downloads else {
            return Err("npm-stat response entry was not an object".to_string());
        };
        for (_, count) in days {
            let n = count
                .as_text()
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or("npm-stat response contained a non-numeric count")?;
            total += n;
        }
    }
    Ok(total)
}

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let today_days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the unix epoch".to_string())?
        .as_secs() as i64
        / 86_400;
    resolve_downloads_at(params, fetcher, today_days)
}

fn resolve_downloads_at(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
    today_days: i64,
) -> Result<String, String> {
    let author = params
        .get("author")
        .ok_or("npm-stat-downloads requires a data-author attribute")?;
    let author = validate_path_param("author", author)?;
    let interval = params
        .get("interval")
        .ok_or("npm-stat-downloads requires a data-interval attribute")?;

    let (y, m, d) = civil_from_days(today_days);
    let until = format_ymd(y, m, d);
    let from = one_unit_before(today_days, interval)?;

    let url = format!(
        "https://npm-stat.com/api/download-counts?author={author}&from={from}&until={until}"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "npm-stat response was not valid UTF-8".to_string())?;
    let doc = crate::json::parse(&text)?;
    let total = total_downloads(&doc)?;
    Ok(total.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY_DAYS: i64 = 19_737; // 2024-01-15

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

    fn params(author: &str, interval: &str) -> HashMap<String, String> {
        HashMap::from([
            ("author".to_string(), author.to_string()),
            ("interval".to_string(), interval.to_string()),
        ])
    }

    #[test]
    fn sums_downloads_across_packages_and_days_for_a_week() {
        let fetcher = FakeFetcher {
            expected_url: "https://npm-stat.com/api/download-counts?author=dukeluo&from=2024-01-08&until=2024-01-15",
            body: r#"{"pkg-a": {"2024-01-09": 5, "2024-01-10": 3}, "pkg-b": {"2024-01-11": 2}}"#,
        };
        let value = resolve_downloads_at(&params("dukeluo", "dw"), &fetcher, TODAY_DAYS).unwrap();
        assert_eq!(value, "10");
    }

    #[test]
    fn computes_a_month_ago_with_calendar_subtraction() {
        let fetcher = FakeFetcher {
            expected_url: "https://npm-stat.com/api/download-counts?author=dukeluo&from=2023-12-15&until=2024-01-15",
            body: r#"{"pkg-a": {"2024-01-01": 100}}"#,
        };
        let value = resolve_downloads_at(&params("dukeluo", "dm"), &fetcher, TODAY_DAYS).unwrap();
        assert_eq!(value, "100");
    }

    #[test]
    fn computes_a_year_ago() {
        let fetcher = FakeFetcher {
            expected_url: "https://npm-stat.com/api/download-counts?author=dukeluo&from=2023-01-15&until=2024-01-15",
            body: r#"{"pkg-a": {"2024-01-01": 7}}"#,
        };
        let value = resolve_downloads_at(&params("dukeluo", "dy"), &fetcher, TODAY_DAYS).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn requires_author_and_interval_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads_at(&HashMap::new(), &Unused, TODAY_DAYS).is_err());
        assert!(resolve_downloads_at(&params("", "dw"), &Unused, TODAY_DAYS).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid author")
            }
        }
        assert!(resolve_downloads_at(&params("../etc/passwd", "dw"), &Unused, TODAY_DAYS).is_err());
    }

    #[test]
    fn rejects_unknown_intervals() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an unknown interval")
            }
        }
        assert!(resolve_downloads_at(&params("dukeluo", "decade"), &Unused, TODAY_DAYS).is_err());
    }

    #[test]
    fn errors_on_a_malformed_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://npm-stat.com/api/download-counts?author=dukeluo&from=2024-01-08&until=2024-01-15",
            body: r#"[1, 2, 3]"#,
        };
        assert!(resolve_downloads_at(&params("dukeluo", "dw"), &fetcher, TODAY_DAYS).is_err());
    }
}
