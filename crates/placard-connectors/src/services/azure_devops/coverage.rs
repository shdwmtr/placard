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

pub(crate) fn resolve_coverage(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let organization = params
        .get("organization")
        .ok_or("azure-devops-coverage requires a data-organization attribute")?;
    let project = params
        .get("project")
        .ok_or("azure-devops-coverage requires a data-project attribute")?;
    let definition_id = params
        .get("definition-id")
        .ok_or("azure-devops-coverage requires a data-definition-id attribute")?;
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
        "https://dev.azure.com/{organization}/{project}/_apis/test/codecoverage?buildId={build_id}&api-version=5.0-preview.1"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "azure devops response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let coverage_data = match value.get("coverageData") {
        Some(Value::Array(items)) => items,
        _ => return Err("azure devops response missing coverageData".to_string()),
    };

    let mut covered = 0.0;
    let mut total = 0.0;
    for entry in coverage_data {
        let Some(Value::Array(stats)) = entry.get("coverageStats") else {
            continue;
        };
        for stat in stats {
            let label = stat.get("label").and_then(|v| v.as_text());
            if label.as_deref() != Some("Line") && label.as_deref() != Some("Lines") {
                continue;
            }
            let stat_covered = match stat.get("covered") {
                Some(Value::Number(n)) => *n,
                _ => return Err("azure devops coverage stat missing covered".to_string()),
            };
            let stat_total = match stat.get("total") {
                Some(Value::Number(n)) => *n,
                _ => return Err("azure devops coverage stat missing total".to_string()),
            };
            covered += stat_covered;
            total += stat_total;
        }
    }

    let coverage = if covered > 0.0 {
        (covered / total) * 100.0
    } else {
        0.0
    };
    Ok(format!("{:.0}%", coverage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeFetcher {
        build_response: &'static str,
        coverage_response: &'static str,
        calls: AtomicUsize,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => {
                    assert_eq!(
                        url,
                        "https://dev.azure.com/swellaby/opensource/_apis/build/builds?definitions=25&$top=1&statusFilter=completed&api-version=5.0-preview.4"
                    );
                    Ok(self.build_response.as_bytes().to_vec())
                }
                1 => {
                    assert_eq!(
                        url,
                        "https://dev.azure.com/swellaby/opensource/_apis/test/codecoverage?buildId=42&api-version=5.0-preview.1"
                    );
                    Ok(self.coverage_response.as_bytes().to_vec())
                }
                _ => panic!("unexpected extra fetch"),
            }
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("organization".to_string(), "swellaby".to_string()),
            ("project".to_string(), "opensource".to_string()),
            ("definition-id".to_string(), "25".to_string()),
        ])
    }

    #[test]
    fn extracts_the_line_coverage_percentage() {
        let fetcher = FakeFetcher {
            build_response: r#"{"count": 1, "value": [{"id": 42}]}"#,
            coverage_response: r#"{"coverageData": [{"coverageStats": [
                {"label": "Lines", "total": 100, "covered": 80},
                {"label": "Blocks", "total": 50, "covered": 10}
            ]}]}"#,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_coverage(&params(), &fetcher).unwrap();
        assert_eq!(value, "80%");
    }

    #[test]
    fn requires_organization_project_and_definition_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_coverage(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("project".to_string(), String::new());
        assert!(resolve_coverage(&p, &Unused).is_err());
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
        p.insert("organization".to_string(), "../etc".to_string());
        assert!(resolve_coverage(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_no_completed_build_is_found() {
        let fetcher = FakeFetcher {
            build_response: r#"{"count": 0, "value": []}"#,
            coverage_response: "",
            calls: AtomicUsize::new(0),
        };
        assert!(resolve_coverage(&params(), &fetcher).is_err());
    }

    #[test]
    fn returns_zero_percent_when_nothing_is_covered() {
        let fetcher = FakeFetcher {
            build_response: r#"{"count": 1, "value": [{"id": 42}]}"#,
            coverage_response: r#"{"coverageData": [{"coverageStats": [
                {"label": "Lines", "total": 0, "covered": 0}
            ]}]}"#,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_coverage(&params(), &fetcher).unwrap();
        assert_eq!(value, "0%");
    }
}
