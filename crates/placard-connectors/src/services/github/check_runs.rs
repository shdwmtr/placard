use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_check_runs(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-check-runs requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-check-runs requires a data-repo attribute")?;
    let git_ref = params
        .get("ref")
        .ok_or("github-check-runs requires a data-ref attribute")?;
    let name = params
        .get("name")
        .ok_or("github-check-runs requires a data-name attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let git_ref = validate_path_param("ref", git_ref)?;
    if name.is_empty() {
        return Err("'name' parameter must not be empty".to_string());
    }

    let url = format!("https://api.github.com/repos/{owner}/{repo}/commits/{git_ref}/check-runs");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let runs = match value.get("check_runs") {
        Some(Value::Array(items)) => items,
        _ => return Err("github response missing check_runs".to_string()),
    };
    let run = runs
        .iter()
        .find(|run| run.get("name").and_then(Value::as_text).as_deref() == Some(name.as_str()))
        .ok_or_else(|| format!("no check run named '{name}' found"))?;
    run.get("conclusion")
        .and_then(Value::as_text)
        .or_else(|| run.get("status").and_then(Value::as_text))
        .ok_or_else(|| "check run was missing conclusion and status".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/badges/shields/commits/master/check-runs"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, git_ref: &str, name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("ref".to_string(), git_ref.to_string()),
            ("name".to_string(), name.to_string()),
        ])
    }

    #[test]
    fn extracts_conclusion_of_the_matching_check_run() {
        let fetcher = FakeFetcher(
            r#"{"total_count": 2, "check_runs": [
                {"name": "lint", "status": "completed", "conclusion": "success"},
                {"name": "test-lint", "status": "completed", "conclusion": "failure"}
            ]}"#,
        );
        let value = resolve_check_runs(
            &params("badges", "shields", "master", "test-lint"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "failure");
    }

    #[test]
    fn falls_back_to_status_when_conclusion_is_null() {
        let fetcher = FakeFetcher(
            r#"{"check_runs": [{"name": "test-lint", "status": "in_progress", "conclusion": null}]}"#,
        );
        let value = resolve_check_runs(
            &params("badges", "shields", "master", "test-lint"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "in_progress");
    }

    #[test]
    fn requires_owner_repo_ref_and_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_check_runs(&HashMap::new(), &Unused).is_err());
        assert!(resolve_check_runs(&params("badges", "shields", "master", ""), &Unused).is_err());
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
            resolve_check_runs(&params("../etc", "shields", "master", "test-lint"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_no_check_run_matches_the_name_filter() {
        let fetcher = FakeFetcher(
            r#"{"check_runs": [{"name": "lint", "status": "completed", "conclusion": "success"}]}"#,
        );
        assert!(
            resolve_check_runs(
                &params("badges", "shields", "master", "test-lint"),
                &fetcher
            )
            .is_err()
        );
    }
}
