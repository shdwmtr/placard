use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_actions_workflow_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-actions-workflow-status requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-actions-workflow-status requires a data-repo attribute")?;
    let workflow = params
        .get("workflow")
        .ok_or("github-actions-workflow-status requires a data-workflow attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let workflow = validate_path_param("workflow", workflow)?;

    let mut url = format!(
        "https://api.github.com/repos/{owner}/{repo}/actions/workflows/{workflow}/runs?per_page=1"
    );
    if let Some(branch) = params.get("branch") {
        let branch = validate_path_param("branch", branch)?;
        url.push_str(&format!("&branch={branch}"));
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let runs = match value.get("workflow_runs") {
        Some(Value::Array(items)) => items,
        _ => return Err("github response missing workflow_runs".to_string()),
    };
    let run = runs.first().ok_or("no workflow runs found")?;
    run.get("conclusion")
        .and_then(Value::as_text)
        .or_else(|| run.get("status").and_then(Value::as_text))
        .ok_or_else(|| "workflow run was missing conclusion and status".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/actions/toolkit/actions/workflows/unit-tests.yml/runs?per_page=1"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, workflow: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("workflow".to_string(), workflow.to_string()),
        ])
    }

    #[test]
    fn extracts_conclusion_from_the_latest_workflow_run() {
        let fetcher =
            FakeFetcher(r#"{"workflow_runs": [{"status": "completed", "conclusion": "success"}]}"#);
        let value = resolve_actions_workflow_status(
            &params("actions", "toolkit", "unit-tests.yml"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn falls_back_to_status_when_conclusion_is_null() {
        let fetcher =
            FakeFetcher(r#"{"workflow_runs": [{"status": "in_progress", "conclusion": null}]}"#);
        let value = resolve_actions_workflow_status(
            &params("actions", "toolkit", "unit-tests.yml"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "in_progress");
    }

    #[test]
    fn requires_owner_repo_and_workflow_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_actions_workflow_status(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_actions_workflow_status(&params("actions", "toolkit", ""), &Unused).is_err()
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
            resolve_actions_workflow_status(&params("../etc", "toolkit", "ci.yml"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_there_are_no_workflow_runs() {
        let fetcher = FakeFetcher(r#"{"workflow_runs": []}"#);
        assert!(
            resolve_actions_workflow_status(
                &params("actions", "toolkit", "unit-tests.yml"),
                &fetcher
            )
            .is_err()
        );
    }
}
