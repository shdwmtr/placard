use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_build(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let organization = params
        .get("organization")
        .ok_or("azure-devops-build requires a data-organization attribute")?;
    let project_id = params
        .get("project-id")
        .ok_or("azure-devops-build requires a data-project-id attribute")?;
    let definition_id = params
        .get("definition-id")
        .ok_or("azure-devops-build requires a data-definition-id attribute")?;
    let organization = validate_path_param("organization", organization)?;
    let project_id = validate_path_param("project-id", project_id)?;
    let definition_id = validate_path_param("definition-id", definition_id)?;

    let mut url = format!(
        "https://dev.azure.com/{organization}/{project_id}/_apis/build/builds?definitions={definition_id}&$top=1&statusFilter=completed&api-version=5.0-preview.4"
    );
    if let Some(branch) = params.get("branch") {
        let branch = validate_path_param("branch", branch)?;
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
        .get("result")
        .ok_or("azure devops response missing result")?
        .as_text()
        .ok_or_else(|| "build result was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn params(
        organization: &str,
        project_id: &str,
        definition_id: &str,
    ) -> HashMap<String, String> {
        HashMap::from([
            ("organization".to_string(), organization.to_string()),
            ("project-id".to_string(), project_id.to_string()),
            ("definition-id".to_string(), definition_id.to_string()),
        ])
    }

    #[test]
    fn extracts_the_build_result() {
        let fetcher = FakeFetcher {
            expected_url: "https://dev.azure.com/totodem/8cf3ec0e-d0c2-4fcd-8206-ad204f254a96/_apis/build/builds?definitions=2&$top=1&statusFilter=completed&api-version=5.0-preview.4",
            body: r#"{"count": 1, "value": [{"id": 42, "status": "completed", "result": "succeeded"}]}"#,
        };
        let value = resolve_build(
            &params("totodem", "8cf3ec0e-d0c2-4fcd-8206-ad204f254a96", "2"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "succeeded");
    }

    #[test]
    fn appends_branch_name_when_provided() {
        let mut p = params("totodem", "8cf3ec0e-d0c2-4fcd-8206-ad204f254a96", "2");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://dev.azure.com/totodem/8cf3ec0e-d0c2-4fcd-8206-ad204f254a96/_apis/build/builds?definitions=2&$top=1&statusFilter=completed&api-version=5.0-preview.4&branchName=refs/heads/master",
            body: r#"{"count": 1, "value": [{"id": 42, "status": "completed", "result": "failed"}]}"#,
        };
        let value = resolve_build(&p, &fetcher).unwrap();
        assert_eq!(value, "failed");
    }

    #[test]
    fn requires_organization_project_id_and_definition_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_build(&HashMap::new(), &Unused).is_err());
        assert!(resolve_build(&params("totodem", "", "2"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_build(&params("../etc", "proj", "2"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_pipeline_is_not_found() {
        let fetcher = FakeFetcher {
            expected_url: "https://dev.azure.com/totodem/proj/_apis/build/builds?definitions=2&$top=1&statusFilter=completed&api-version=5.0-preview.4",
            body: r#"{"count": 0, "value": []}"#,
        };
        assert!(resolve_build(&params("totodem", "proj", "2"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_result_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://dev.azure.com/totodem/proj/_apis/build/builds?definitions=2&$top=1&statusFilter=completed&api-version=5.0-preview.4",
            body: r#"{"count": 1, "value": [{"id": 42, "status": "inProgress"}]}"#,
        };
        assert!(resolve_build(&params("totodem", "proj", "2"), &fetcher).is_err());
    }
}
