use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(*n),
        _ => None,
    }
}

/// Converts a day count since the Unix epoch (1970-01-01) into a
/// `YYYY-MM-DD` string, via Howard Hinnant's `civil_from_days` algorithm.
fn format_ymd(days: i64) -> String {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn format_percentage(uptime: f64) -> String {
    if uptime == 100.0 {
        "100%".to_string()
    } else {
        format!("{uptime:.3}%")
    }
}

/// Reports the most recent daily uptime percentage over a trailing 30-day
/// window, matching what shields' own badge shows.
pub(crate) fn resolve_uptime(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let today_days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the unix epoch".to_string())?
        .as_secs() as i64
        / 86_400;
    resolve_uptime_at(params, fetcher, today_days)
}

fn resolve_uptime_at(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
    today_days: i64,
) -> Result<String, String> {
    let check_uuid = params
        .get("check-uuid")
        .ok_or("nodeping-uptime requires a data-check-uuid attribute")?;
    let check_uuid = validate_path_param("check-uuid", check_uuid)?;

    let start_date = format_ymd(today_days - 30);
    let url = format!(
        "https://nodeping.com/reports/uptime/{check_uuid}?format=json&interval=days&start={start_date}"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "nodeping response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let Value::Array(rows) = &value else {
        return Err("nodeping response was not an array".to_string());
    };
    let last = rows.last().ok_or("nodeping response had no result rows")?;
    let uptime = field(last, "uptime")
        .and_then(as_number)
        .ok_or("nodeping response missing uptime field")?;
    Ok(format_percentage(uptime))
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

    fn params(check_uuid: &str) -> HashMap<String, String> {
        HashMap::from([("check-uuid".to_string(), check_uuid.to_string())])
    }

    #[test]
    fn formats_the_epoch_correctly() {
        assert_eq!(format_ymd(0), "1970-01-01");
    }

    #[test]
    fn extracts_the_latest_row_and_formats_a_partial_percentage() {
        let fetcher = FakeFetcher {
            expected_url: "https://nodeping.com/reports/uptime/jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei?format=json&interval=days&start=2023-12-16",
            body: r#"[{"uptime": 99.5}, {"uptime": 99.987}]"#,
        };
        let value = resolve_uptime_at(
            &params("jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei"),
            &fetcher,
            TODAY_DAYS,
        )
        .unwrap();
        assert_eq!(value, "99.987%");
    }

    #[test]
    fn formats_a_perfect_score_without_decimals() {
        let fetcher = FakeFetcher {
            expected_url: "https://nodeping.com/reports/uptime/jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei?format=json&interval=days&start=2023-12-16",
            body: r#"[{"uptime": 100.0}]"#,
        };
        let value = resolve_uptime_at(
            &params("jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei"),
            &fetcher,
            TODAY_DAYS,
        )
        .unwrap();
        assert_eq!(value, "100%");
    }

    #[test]
    fn requires_check_uuid_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid check-uuid")
            }
        }
        assert!(resolve_uptime_at(&HashMap::new(), &Unused, TODAY_DAYS).is_err());
        assert!(resolve_uptime_at(&params(""), &Unused, TODAY_DAYS).is_err());
    }

    #[test]
    fn rejects_path_breaking_check_uuid_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid check-uuid")
            }
        }
        assert!(resolve_uptime_at(&params("../etc/passwd"), &Unused, TODAY_DAYS).is_err());
    }

    #[test]
    fn errors_when_response_is_empty_or_malformed() {
        let fetcher = FakeFetcher {
            expected_url: "https://nodeping.com/reports/uptime/jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei?format=json&interval=days&start=2023-12-16",
            body: r#"[]"#,
        };
        assert!(
            resolve_uptime_at(
                &params("jkiwn052-ntpp-4lbb-8d45-ihew6d9ucoei"),
                &fetcher,
                TODAY_DAYS
            )
            .is_err()
        );
    }
}
