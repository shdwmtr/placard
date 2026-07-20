use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn as_number(value: &Value, field: &str) -> Result<f64, String> {
    match value.get(field) {
        Some(Value::Number(n)) => Ok(*n),
        _ => Err(format!("github response missing {field}")),
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

pub(crate) fn resolve_milestone_detail(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-milestone-detail requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-milestone-detail requires a data-repo attribute")?;
    let number = params
        .get("number")
        .ok_or("github-milestone-detail requires a data-number attribute")?;
    let variant = params
        .get("variant")
        .ok_or("github-milestone-detail requires a data-variant attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let number = validate_path_param("number", number)?;
    if !matches!(
        variant.as_str(),
        "issues-open" | "issues-closed" | "issues-total" | "progress" | "progress-percent"
    ) {
        return Err(format!(
            "'variant' parameter '{variant}' is not one of issues-open, issues-closed, issues-total, progress, progress-percent"
        ));
    }

    let url = format!("https://api.github.com/repos/{owner}/{repo}/milestones/{number}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let open_issues = as_number(&value, "open_issues")?;
    let closed_issues = as_number(&value, "closed_issues")?;

    Ok(match variant.as_str() {
        "issues-open" => format_number(open_issues),
        "issues-closed" => format_number(closed_issues),
        "issues-total" => format_number(open_issues + closed_issues),
        "progress" => format!(
            "{}/{}",
            format_number(closed_issues),
            format_number(open_issues + closed_issues)
        ),
        "progress-percent" => {
            let total = open_issues + closed_issues;
            if total == 0.0 {
                return Err("milestone has no issues".to_string());
            }
            format!("{}%", ((closed_issues / total) * 100.0).floor() as i64)
        }
        _ => unreachable!(),
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
                "https://api.github.com/repos/badges/shields/milestones/1"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(variant: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "badges".to_string()),
            ("repo".to_string(), "shields".to_string()),
            ("number".to_string(), "1".to_string()),
            ("variant".to_string(), variant.to_string()),
        ])
    }

    #[test]
    fn extracts_open_issues() {
        let fetcher = FakeFetcher(r#"{"title": "v1", "open_issues": 3, "closed_issues": 7}"#);
        let value = resolve_milestone_detail(&params("issues-open"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn extracts_closed_issues() {
        let fetcher = FakeFetcher(r#"{"title": "v1", "open_issues": 3, "closed_issues": 7}"#);
        let value = resolve_milestone_detail(&params("issues-closed"), &fetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn sums_open_and_closed_for_issues_total() {
        let fetcher = FakeFetcher(r#"{"title": "v1", "open_issues": 3, "closed_issues": 7}"#);
        let value = resolve_milestone_detail(&params("issues-total"), &fetcher).unwrap();
        assert_eq!(value, "10");
    }

    #[test]
    fn formats_progress_as_a_fraction() {
        let fetcher = FakeFetcher(r#"{"title": "v1", "open_issues": 3, "closed_issues": 7}"#);
        let value = resolve_milestone_detail(&params("progress"), &fetcher).unwrap();
        assert_eq!(value, "7/10");
    }

    #[test]
    fn formats_progress_percent() {
        let fetcher = FakeFetcher(r#"{"title": "v1", "open_issues": 1, "closed_issues": 3}"#);
        let value = resolve_milestone_detail(&params("progress-percent"), &fetcher).unwrap();
        assert_eq!(value, "75%");
    }

    #[test]
    fn requires_owner_repo_number_and_variant_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_milestone_detail(&HashMap::new(), &Unused).is_err());
        let mut p = params("issues-open");
        p.insert("number".to_string(), String::new());
        assert!(resolve_milestone_detail(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_an_unknown_variant() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid variant")
            }
        }
        assert!(resolve_milestone_detail(&params("bogus"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params("issues-open");
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_milestone_detail(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_fields_are_missing() {
        let fetcher = FakeFetcher(r#"{"title": "v1"}"#);
        assert!(resolve_milestone_detail(&params("issues-open"), &fetcher).is_err());
    }
}
