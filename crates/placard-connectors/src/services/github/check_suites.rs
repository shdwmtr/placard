use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_check_suites(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-check-suites requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-check-suites requires a data-repo attribute")?;
    let git_ref = params
        .get("ref")
        .ok_or("github-check-suites requires a data-ref attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let git_ref = validate_path_param("ref", git_ref)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/commits/{git_ref}/check-suites");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let suites = match value.get("check_suites") {
        Some(Value::Array(items)) => items,
        _ => return Err("github response missing check_suites".to_string()),
    };
    let suite = suites
        .iter()
        .find(|suite| matches!(suite.get("latest_check_runs_count"), Some(Value::Number(n)) if *n > 0.0))
        .ok_or("no check suites with check runs found")?;
    suite
        .get("conclusion")
        .and_then(Value::as_text)
        .or_else(|| suite.get("status").and_then(Value::as_text))
        .ok_or_else(|| "check suite was missing conclusion and status".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/badges/shields/commits/master/check-suites"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, git_ref: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("ref".to_string(), git_ref.to_string()),
        ])
    }

    #[test]
    fn extracts_conclusion_of_the_first_suite_with_check_runs() {
        let fetcher = FakeFetcher(
            r#"{"check_suites": [
                {"status": "completed", "conclusion": null, "latest_check_runs_count": 0},
                {"status": "completed", "conclusion": "success", "latest_check_runs_count": 3}
            ]}"#,
        );
        let value = resolve_check_suites(&params("badges", "shields", "master"), &fetcher).unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn falls_back_to_status_when_conclusion_is_null() {
        let fetcher = FakeFetcher(
            r#"{"check_suites": [{"status": "in_progress", "conclusion": null, "latest_check_runs_count": 2}]}"#,
        );
        let value = resolve_check_suites(&params("badges", "shields", "master"), &fetcher).unwrap();
        assert_eq!(value, "in_progress");
    }

    #[test]
    fn requires_owner_repo_and_ref_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_check_suites(&HashMap::new(), &Unused).is_err());
        assert!(resolve_check_suites(&params("badges", "shields", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_check_suites(&params("../etc", "shields", "master"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_suite_has_check_runs() {
        let fetcher = FakeFetcher(
            r#"{"check_suites": [{"status": "completed", "conclusion": "success", "latest_check_runs_count": 0}]}"#,
        );
        assert!(resolve_check_suites(&params("badges", "shields", "master"), &fetcher).is_err());
    }
}
