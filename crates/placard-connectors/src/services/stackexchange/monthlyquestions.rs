use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Stack Exchange's questions endpoint needs an explicit `fromdate`/`todate`
/// range, so this builds one covering the previous calendar month (the
/// current month is still in progress), via Howard Hinnant's civil calendar
/// algorithms for converting between a day count since the Unix epoch and a
/// (year, month, day) triple.
pub(crate) fn resolve_monthlyquestions(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let today_days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the unix epoch".to_string())?
        .as_secs() as i64
        / 86_400;
    resolve_monthlyquestions_at(params, fetcher, today_days)
}

fn resolve_monthlyquestions_at(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
    today_days: i64,
) -> Result<String, String> {
    let site = params
        .get("stackexchangesite")
        .ok_or("stackexchange-monthlyquestions requires a data-stackexchangesite attribute")?;
    let query = params
        .get("query")
        .ok_or("stackexchange-monthlyquestions requires a data-query attribute")?;
    let site = validate_path_param("stackexchangesite", site)?;
    let query = validate_path_param("query", query)?;

    let (year, month, _) = civil_from_days(today_days);
    let total_months = year * 12 + (month - 1);
    let prev_total_months = total_months - 1;
    let (prev_year, prev_month) = (
        prev_total_months.div_euclid(12),
        prev_total_months.rem_euclid(12) + 1,
    );
    let (this_year, this_month) = (total_months.div_euclid(12), total_months.rem_euclid(12) + 1);

    let start_days = days_from_civil(prev_year, prev_month, 1);
    let end_days = days_from_civil(this_year, this_month, 1) - 1;
    let fromdate = start_days * 86_400;
    let todate = end_days * 86_400 + 86_399;

    let url = format!(
        "https://api.stackexchange.com/2.2/questions?site={site}&fromdate={fromdate}&todate={todate}&filter=total&tagged={query}"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "stackexchange response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let total = value
        .get("total")
        .ok_or("stackexchange response missing total")?;
    total
        .as_text()
        .ok_or_else(|| "total was not a plain value".to_string())
}

/// Converts a day count since the Unix epoch (1970-01-01) into a
/// `(year, month, day)` triple, via Howard Hinnant's `civil_from_days`
/// algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i64;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as i64;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Converts a `(year, month, day)` triple into a day count since the Unix
/// epoch, the inverse of [`civil_from_days`].
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doy = doy as u64;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY_DAYS: i64 = 19_737; // 2024-01-15

    struct FakeFetcher(String, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    fn params(site: &str, query: &str) -> HashMap<String, String> {
        HashMap::from([
            ("stackexchangesite".to_string(), site.to_string()),
            ("query".to_string(), query.to_string()),
        ])
    }

    #[test]
    fn civil_from_days_and_days_from_civil_round_trip() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(TODAY_DAYS), (2024, 1, 15));
        assert_eq!(days_from_civil(2024, 1, 15), TODAY_DAYS);
    }

    #[test]
    fn builds_a_range_covering_the_previous_calendar_month() {
        // Previous month for 2024-01-15 is December 2023: Dec 1 00:00:00 UTC
        // through Dec 31 23:59:59 UTC.
        let fromdate = days_from_civil(2023, 12, 1) * 86_400;
        let todate = days_from_civil(2024, 1, 1) * 86_400 - 1;
        let fetcher = FakeFetcher(
            format!(
                "https://api.stackexchange.com/2.2/questions?site=stackoverflow&fromdate={fromdate}&todate={todate}&filter=total&tagged=javascript"
            ),
            r#"{"total": 4821}"#,
        );
        let value = resolve_monthlyquestions_at(
            &params("stackoverflow", "javascript"),
            &fetcher,
            TODAY_DAYS,
        )
        .unwrap();
        assert_eq!(value, "4821");
    }

    #[test]
    fn rolls_over_the_year_boundary_when_the_previous_month_is_december() {
        // 2024-01-01 -> previous month is December 2023.
        let jan_1_2024 = days_from_civil(2024, 1, 1);
        let fromdate = days_from_civil(2023, 12, 1) * 86_400;
        let todate = days_from_civil(2024, 1, 1) * 86_400 - 1;
        let fetcher = FakeFetcher(
            format!(
                "https://api.stackexchange.com/2.2/questions?site=stackoverflow&fromdate={fromdate}&todate={todate}&filter=total&tagged=rust"
            ),
            r#"{"total": 12}"#,
        );
        let value =
            resolve_monthlyquestions_at(&params("stackoverflow", "rust"), &fetcher, jan_1_2024)
                .unwrap();
        assert_eq!(value, "12");
    }

    #[test]
    fn requires_site_and_query_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_monthlyquestions_at(&HashMap::new(), &Unused, TODAY_DAYS).is_err());
        assert!(
            resolve_monthlyquestions_at(&params("stackoverflow", ""), &Unused, TODAY_DAYS).is_err()
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
        assert!(
            resolve_monthlyquestions_at(&params("stackoverflow", "../etc"), &Unused, TODAY_DAYS)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_total_field_is_missing() {
        let fromdate = days_from_civil(2023, 12, 1) * 86_400;
        let todate = days_from_civil(2024, 1, 1) * 86_400 - 1;
        let fetcher = FakeFetcher(
            format!(
                "https://api.stackexchange.com/2.2/questions?site=stackoverflow&fromdate={fromdate}&todate={todate}&filter=total&tagged=javascript"
            ),
            r#"{}"#,
        );
        assert!(
            resolve_monthlyquestions_at(
                &params("stackoverflow", "javascript"),
                &fetcher,
                TODAY_DAYS
            )
            .is_err()
        );
    }
}
