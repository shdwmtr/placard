use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// SourceForge's download-stats endpoint requires an explicit date range, so
/// this builds one ending "yesterday" (today is always incomplete) sized by
/// `data-interval`: a day, a week, a month, or since the epoch (all-time).
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
    let project = params
        .get("project")
        .ok_or("sourceforge-downloads requires a data-project attribute")?;
    let project = validate_path_param("project", project)?;
    let interval = match params.get("interval").map(String::as_str) {
        Some(i @ ("dd" | "dw" | "dm" | "dt")) => i,
        Some(_) => {
            return Err(
                "sourceforge-downloads data-interval must be one of 'dd', 'dw', 'dm', 'dt'"
                    .to_string(),
            );
        }
        None => return Err("sourceforge-downloads requires a data-interval attribute".to_string()),
    };
    let folder = match params.get("folder") {
        Some(f) => Some(validate_path_param("folder", f)?),
        None => None,
    };

    let end_days = today_days - 1;
    let start_days = match interval {
        "dd" => end_days,
        "dw" => end_days - 6,
        "dm" => end_days - 30,
        _ => 0,
    };

    let end_date = format_ymd(end_days);
    let start_date = format_ymd(start_days);
    let folder_segment = folder.map(|f| format!("{f}/")).unwrap_or_default();

    let url = format!(
        "https://sourceforge.net/projects/{project}/files/{folder_segment}stats/json?start_date={start_date}&end_date={end_date}"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourceforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let total = value
        .get("total")
        .ok_or("sourceforge response missing total")?;
    total
        .as_text()
        .ok_or_else(|| "total was not a plain value".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY_DAYS: i64 = 19_737; // 2024-01-15

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    fn params(project: &str, interval: &str, folder: Option<&str>) -> HashMap<String, String> {
        let mut map = HashMap::from([
            ("project".to_string(), project.to_string()),
            ("interval".to_string(), interval.to_string()),
        ]);
        if let Some(folder) = folder {
            map.insert("folder".to_string(), folder.to_string());
        }
        map
    }

    #[test]
    fn formats_the_epoch_correctly() {
        assert_eq!(format_ymd(0), "1970-01-01");
    }

    #[test]
    fn builds_a_single_day_range_for_dd() {
        let fetcher = FakeFetcher(
            "https://sourceforge.net/projects/sevenzip/files/stats/json?start_date=2024-01-14&end_date=2024-01-14",
            r#"{"total": 123}"#,
        );
        let value =
            resolve_downloads_at(&params("sevenzip", "dd", None), &fetcher, TODAY_DAYS).unwrap();
        assert_eq!(value, "123");
    }

    #[test]
    fn builds_a_seven_day_range_for_dw() {
        let fetcher = FakeFetcher(
            "https://sourceforge.net/projects/sevenzip/files/stats/json?start_date=2024-01-08&end_date=2024-01-14",
            r#"{"total": 456}"#,
        );
        let value =
            resolve_downloads_at(&params("sevenzip", "dw", None), &fetcher, TODAY_DAYS).unwrap();
        assert_eq!(value, "456");
    }

    #[test]
    fn builds_a_thirty_one_day_range_for_dm() {
        let fetcher = FakeFetcher(
            "https://sourceforge.net/projects/sevenzip/files/stats/json?start_date=2023-12-15&end_date=2024-01-14",
            r#"{"total": 789}"#,
        );
        let value =
            resolve_downloads_at(&params("sevenzip", "dm", None), &fetcher, TODAY_DAYS).unwrap();
        assert_eq!(value, "789");
    }

    #[test]
    fn builds_an_all_time_range_for_dt() {
        let fetcher = FakeFetcher(
            "https://sourceforge.net/projects/sevenzip/files/stats/json?start_date=1970-01-01&end_date=2024-01-14",
            r#"{"total": 999999}"#,
        );
        let value =
            resolve_downloads_at(&params("sevenzip", "dt", None), &fetcher, TODAY_DAYS).unwrap();
        assert_eq!(value, "999999");
    }

    #[test]
    fn includes_the_folder_segment_when_provided() {
        let fetcher = FakeFetcher(
            "https://sourceforge.net/projects/arianne/files/stendhal/stats/json?start_date=2024-01-14&end_date=2024-01-14",
            r#"{"total": 42}"#,
        );
        let value = resolve_downloads_at(
            &params("arianne", "dd", Some("stendhal")),
            &fetcher,
            TODAY_DAYS,
        )
        .unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_project_and_interval_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads_at(&HashMap::new(), &Unused, TODAY_DAYS).is_err());
        assert!(resolve_downloads_at(&params("sevenzip", "", None), &Unused, TODAY_DAYS).is_err());
    }

    #[test]
    fn rejects_an_unknown_interval() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid interval")
            }
        }
        assert!(
            resolve_downloads_at(&params("sevenzip", "bogus", None), &Unused, TODAY_DAYS).is_err()
        );
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads_at(&params("../etc", "dd", None), &Unused, TODAY_DAYS).is_err());
    }

    #[test]
    fn errors_when_the_total_field_is_missing() {
        let fetcher = FakeFetcher(
            "https://sourceforge.net/projects/sevenzip/files/stats/json?start_date=2024-01-14&end_date=2024-01-14",
            r#"{}"#,
        );
        assert!(
            resolve_downloads_at(&params("sevenzip", "dd", None), &fetcher, TODAY_DAYS).is_err()
        );
    }
}
