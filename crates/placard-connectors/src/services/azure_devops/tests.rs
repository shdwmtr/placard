use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn latest_completed_build_id(
    organization: &str,
    project: &str,
    definition_id: &str,
    branch: Option<&str>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let mut url = format!(
        "https://dev.azure.com/{organization}/{project}/_apis/build/builds?definitions={definition_id}&$top=1&statusFilter=completed&api-version=5.0-preview.4"
    );
    if let Some(branch) = branch {
        url.push_str(&format!("&branchName=refs/heads/{branch}"));
    }

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "azure devops response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = match value.get("count") {
        Some(Value::Number(n)) => *n,
        _ => return Err("azure devops response missing count".to_string()),
    };
    let items = match value.get("value") {
        Some(Value::Array(items)) => items,
        _ => return Err("azure devops response missing value array".to_string()),
    };
    if count != 1.0 || items.len() != 1 {
        return Err("build pipeline not found".to_string());
    }
    items[0]
        .get("id")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "azure devops response missing build id".to_string())
}

fn outcome_count(by_outcome: &Value, name: &str) -> i64 {
    match by_outcome.get(&format!("{name}.count")) {
        Some(Value::Number(n)) => *n as i64,
        _ => 0,
    }
}

pub(crate) fn resolve_tests(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let organization = params
        .get("organization")
        .ok_or("azure-devops-tests requires a data-organization attribute")?;
    let project = params
        .get("project")
        .ok_or("azure-devops-tests requires a data-project attribute")?;
    let definition_id = params
        .get("definition-id")
        .ok_or("azure-devops-tests requires a data-definition-id attribute")?;
    let organization = validate_path_param("organization", organization)?;
    let project = validate_path_param("project", project)?;
    let definition_id = validate_path_param("definition-id", definition_id)?;
    let branch = match params.get("branch") {
        Some(branch) => Some(validate_path_param("branch", branch)?),
        None => None,
    };

    let build_id =
        latest_completed_build_id(organization, project, definition_id, branch, fetcher)?;

    let url = format!(
        "https://dev.azure.com/{organization}/{project}/_apis/test/ResultSummaryByBuild?buildId={build_id}"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "azure devops response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let analysis = value
        .get("aggregatedResultsAnalysis")
        .ok_or("azure devops response missing aggregatedResultsAnalysis")?;
    let total = match analysis.get("totalTests") {
        Some(Value::Number(n)) => *n as i64,
        _ => return Err("azure devops response missing totalTests".to_string()),
    };
    let by_outcome = analysis
        .get("resultsByOutcome")
        .ok_or("azure devops response missing resultsByOutcome")?;
    let passed = outcome_count(by_outcome, "Passed");
    let failed = outcome_count(by_outcome, "Failed");
    let skipped = total - passed - failed;

    let mut parts = vec![format!("{passed} passed")];
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    if skipped > 0 {
        parts.push(format!("{skipped} skipped"));
    }
    Ok(parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeFetcher {
        build_response: &'static str,
        summary_response: &'static str,
        calls: AtomicUsize,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => {
                    assert_eq!(
                        url,
                        "https://dev.azure.com/azuredevops-powershell/azuredevops-powershell/_apis/build/builds?definitions=1&$top=1&statusFilter=completed&api-version=5.0-preview.4"
                    );
                    Ok(self.build_response.as_bytes().to_vec())
                }
                1 => {
                    assert_eq!(
                        url,
                        "https://dev.azure.com/azuredevops-powershell/azuredevops-powershell/_apis/test/ResultSummaryByBuild?buildId=20"
                    );
                    Ok(self.summary_response.as_bytes().to_vec())
                }
                _ => panic!("unexpected extra fetch"),
            }
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([
            (
                "organization".to_string(),
                "azuredevops-powershell".to_string(),
            ),
            ("project".to_string(), "azuredevops-powershell".to_string()),
            ("definition-id".to_string(), "1".to_string()),
        ])
    }

    #[test]
    fn extracts_passed_failed_and_skipped_counts() {
        let fetcher = FakeFetcher {
            build_response: r#"{"count": 1, "value": [{"id": 20}]}"#,
            summary_response: r#"{"aggregatedResultsAnalysis": {"totalTests": 10, "resultsByOutcome": {
                "Passed": {"count": 7},
                "Failed": {"count": 1}
            }}}"#,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_tests(&params(), &fetcher).unwrap();
        assert_eq!(value, "7 passed, 1 failed, 2 skipped");
    }

    #[test]
    fn omits_failed_and_skipped_when_zero() {
        let fetcher = FakeFetcher {
            build_response: r#"{"count": 1, "value": [{"id": 20}]}"#,
            summary_response: r#"{"aggregatedResultsAnalysis": {"totalTests": 5, "resultsByOutcome": {
                "Passed": {"count": 5}
            }}}"#,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_tests(&params(), &fetcher).unwrap();
        assert_eq!(value, "5 passed");
    }

    #[test]
    fn requires_organization_project_and_definition_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_tests(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("definition-id".to_string(), String::new());
        assert!(resolve_tests(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params();
        p.insert("project".to_string(), "../etc".to_string());
        assert!(resolve_tests(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_no_completed_build_is_found() {
        let fetcher = FakeFetcher {
            build_response: r#"{"count": 0, "value": []}"#,
            summary_response: "",
            calls: AtomicUsize::new(0),
        };
        assert!(resolve_tests(&params(), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_summary_is_missing_the_analysis() {
        let fetcher = FakeFetcher {
            build_response: r#"{"count": 1, "value": [{"id": 20}]}"#,
            summary_response: r#"{}"#,
            calls: AtomicUsize::new(0),
        };
        assert!(resolve_tests(&params(), &fetcher).is_err());
    }
}
