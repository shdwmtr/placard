use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub(crate) fn resolve_pipelines(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("bitbucket-pipelines requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("bitbucket-pipelines requires a data-repo attribute")?;
    let branch = params
        .get("branch")
        .ok_or("bitbucket-pipelines requires a data-branch attribute")?;
    if branch.is_empty() {
        return Err("'branch' parameter must not be empty".to_string());
    }
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!(
        "https://api.bitbucket.org/2.0/repositories/{user}/{repo}/pipelines/?fields=values.state&page=1&pagelen=2&sort=-created_on&target.ref_type=BRANCH&target.ref_name={}",
        percent_encode(branch)
    );

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "bitbucket response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let values = value
        .get("values")
        .ok_or("bitbucket response missing values")?;
    let Value::Array(items) = values else {
        return Err("bitbucket response values was not an array".to_string());
    };

    let completed = items.iter().find(|item| {
        item.get("state.name").and_then(Value::as_text).as_deref() == Some("COMPLETED")
    });

    match completed {
        Some(item) => item
            .get("state.result.name")
            .and_then(Value::as_text)
            .ok_or_else(|| "bitbucket pipeline entry missing state.result.name".to_string()),
        None => Ok("never built".to_string()),
    }
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

    fn params(user: &str, repo: &str, branch: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("branch".to_string(), branch.to_string()),
        ])
    }

    const EXPECTED_URL: &str = "https://api.bitbucket.org/2.0/repositories/shields-io/test-repo/pipelines/?fields=values.state&page=1&pagelen=2&sort=-created_on&target.ref_type=BRANCH&target.ref_name=main";

    #[test]
    fn extracts_the_result_of_the_latest_completed_pipeline() {
        let fetcher = FakeFetcher {
            expected_url: EXPECTED_URL,
            body: r#"{"values": [{"state": {"name": "COMPLETED", "result": {"name": "SUCCESSFUL"}}}]}"#,
        };
        let value =
            resolve_pipelines(&params("shields-io", "test-repo", "main"), &fetcher).unwrap();
        assert_eq!(value, "SUCCESSFUL");
    }

    #[test]
    fn skips_non_completed_entries() {
        let fetcher = FakeFetcher {
            expected_url: EXPECTED_URL,
            body: r#"{"values": [{"state": {"name": "IN_PROGRESS"}}, {"state": {"name": "COMPLETED", "result": {"name": "FAILED"}}}]}"#,
        };
        let value =
            resolve_pipelines(&params("shields-io", "test-repo", "main"), &fetcher).unwrap();
        assert_eq!(value, "FAILED");
    }

    #[test]
    fn returns_never_built_when_nothing_has_completed() {
        let fetcher = FakeFetcher {
            expected_url: EXPECTED_URL,
            body: r#"{"values": [{"state": {"name": "IN_PROGRESS"}}]}"#,
        };
        let value =
            resolve_pipelines(&params("shields-io", "test-repo", "main"), &fetcher).unwrap();
        assert_eq!(value, "never built");
    }

    #[test]
    fn requires_user_repo_and_branch_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_pipelines(&HashMap::new(), &Unused).is_err());
        assert!(resolve_pipelines(&params("shields-io", "test-repo", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_pipelines(&params("../etc", "test-repo", "main"), &Unused).is_err());
    }

    #[test]
    fn errors_when_a_completed_entry_is_missing_the_result_name() {
        let fetcher = FakeFetcher {
            expected_url: EXPECTED_URL,
            body: r#"{"values": [{"state": {"name": "COMPLETED"}}]}"#,
        };
        assert!(resolve_pipelines(&params("shields-io", "test-repo", "main"), &fetcher).is_err());
    }
}
