use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_cii_best_practices(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_id = params
        .get("project-id")
        .ok_or("cii-best-practices requires a data-project-id attribute")?;
    let project_id = validate_path_param("project-id", project_id)?;
    let metric = params.get("metric").map(String::as_str).unwrap_or("level");
    if !matches!(metric, "level" | "percentage" | "summary") {
        return Err(format!(
            "'metric' parameter must be one of level, percentage, summary, got '{metric}'"
        ));
    }

    let url =
        format!("https://bestpractices.coreinfrastructure.org/projects/{project_id}/badge.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "cii-best-practices response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    if metric == "level" {
        let level = value
            .get("badge_level")
            .ok_or("cii-best-practices response missing badge_level")?;
        let level = level
            .as_text()
            .ok_or_else(|| "badge_level was not a plain value".to_string())?;
        return Ok(if level == "in_progress" {
            "in progress".to_string()
        } else {
            level
        });
    }

    let percentage = value
        .get("tiered_percentage")
        .ok_or("cii-best-practices response missing tiered_percentage")?;
    let percentage = percentage
        .as_text()
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| "tiered_percentage was not a numeric value".to_string())?;

    if metric == "percentage" {
        return Ok(format!("{}%", percentage.round() as i64));
    }

    Ok(if percentage < 100.0 {
        format!("in progress {}%", percentage as i64)
    } else if percentage < 200.0 {
        "passing".to_string()
    } else if percentage < 300.0 {
        "silver".to_string()
    } else {
        "gold".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://bestpractices.coreinfrastructure.org/projects/1/badge.json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(project_id: &str, metric: Option<&str>) -> HashMap<String, String> {
        let mut map = HashMap::from([("project-id".to_string(), project_id.to_string())]);
        if let Some(metric) = metric {
            map.insert("metric".to_string(), metric.to_string());
        }
        map
    }

    #[test]
    fn extracts_level_by_default() {
        let fetcher = FakeFetcher(r#"{"badge_level": "passing", "tiered_percentage": 100}"#);
        let value = resolve_cii_best_practices(&params("1", None), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn formats_in_progress_level_with_a_space() {
        let fetcher = FakeFetcher(r#"{"badge_level": "in_progress", "tiered_percentage": 45}"#);
        let value = resolve_cii_best_practices(&params("1", Some("level")), &fetcher).unwrap();
        assert_eq!(value, "in progress");
    }

    #[test]
    fn formats_percentage_metric() {
        let fetcher = FakeFetcher(r#"{"badge_level": "passing", "tiered_percentage": 100}"#);
        let value = resolve_cii_best_practices(&params("1", Some("percentage")), &fetcher).unwrap();
        assert_eq!(value, "100%");
    }

    #[test]
    fn derives_summary_tiers_from_percentage() {
        let gold = FakeFetcher(r#"{"badge_level": "gold", "tiered_percentage": 305}"#);
        assert_eq!(
            resolve_cii_best_practices(&params("1", Some("summary")), &gold).unwrap(),
            "gold"
        );

        let silver = FakeFetcher(r#"{"badge_level": "silver", "tiered_percentage": 210}"#);
        assert_eq!(
            resolve_cii_best_practices(&params("1", Some("summary")), &silver).unwrap(),
            "silver"
        );

        let in_progress = FakeFetcher(r#"{"badge_level": "in_progress", "tiered_percentage": 45}"#);
        assert_eq!(
            resolve_cii_best_practices(&params("1", Some("summary")), &in_progress).unwrap(),
            "in progress 45%"
        );
    }

    #[test]
    fn requires_a_project_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_cii_best_practices(&HashMap::new(), &Unused).is_err());
        assert!(resolve_cii_best_practices(&params("", None), &Unused).is_err());
    }

    #[test]
    fn rejects_an_invalid_metric() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid metric")
            }
        }
        assert!(resolve_cii_best_practices(&params("1", Some("bogus")), &Unused).is_err());
    }

    #[test]
    fn errors_when_a_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"badge_level": "passing"}"#);
        assert!(resolve_cii_best_practices(&params("1", Some("percentage")), &fetcher).is_err());
    }
}
