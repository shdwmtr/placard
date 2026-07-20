use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_tests(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("appveyor-tests requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("appveyor-tests requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let mut url = format!("https://ci.appveyor.com/api/projects/{user}/{repo}");
    if let Some(branch) = params.get("branch") {
        let branch = validate_path_param("branch", branch)?;
        url.push_str(&format!("/branch/{branch}"));
    }

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "appveyor response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    if value.get("build").is_none() {
        return Ok("no builds found".to_string());
    }

    let jobs = value
        .get("build.jobs")
        .ok_or("appveyor response missing build.jobs")?;
    let Value::Array(items) = jobs else {
        return Err("build.jobs was not an array".to_string());
    };

    let mut total = 0i64;
    let mut passed = 0i64;
    let mut failed = 0i64;
    for item in items {
        total += field_as_i64(item, "testsCount")?;
        passed += field_as_i64(item, "passedTestsCount")?;
        failed += field_as_i64(item, "failedTestsCount")?;
    }
    let skipped = total - passed - failed;

    if total == 0 {
        return Ok("no tests".to_string());
    }

    let mut parts = vec![format!("{passed} passed")];
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    if skipped > 0 {
        parts.push(format!("{skipped} skipped"));
    }
    Ok(parts.join(", "))
}

fn field_as_i64(item: &Value, field: &str) -> Result<i64, String> {
    match item.get(field) {
        Some(Value::Number(n)) => Ok(*n as i64),
        _ => Err(format!("job entry missing {field}")),
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

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn sums_test_counts_across_jobs() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/NZSmartie/coap-net-iu0to",
            body: r#"{"build": {"status": "success", "jobs": [
                {"name": "a", "status": "success", "testsCount": 10, "passedTestsCount": 8, "failedTestsCount": 1},
                {"name": "b", "status": "success", "testsCount": 5, "passedTestsCount": 5, "failedTestsCount": 0}
            ]}}"#,
        };
        let value = resolve_tests(&params("NZSmartie", "coap-net-iu0to"), &fetcher).unwrap();
        assert_eq!(value, "13 passed, 1 failed, 1 skipped");
    }

    #[test]
    fn appends_branch_to_the_url_when_provided() {
        let mut p = params("NZSmartie", "coap-net-iu0to");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/NZSmartie/coap-net-iu0to/branch/master",
            body: r#"{"build": {"status": "success", "jobs": [
                {"name": "a", "status": "success", "testsCount": 2, "passedTestsCount": 2, "failedTestsCount": 0}
            ]}}"#,
        };
        let value = resolve_tests(&p, &fetcher).unwrap();
        assert_eq!(value, "2 passed");
    }

    #[test]
    fn reports_no_builds_found_when_build_key_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/NZSmartie/coap-net-iu0to",
            body: r#"{}"#,
        };
        let value = resolve_tests(&params("NZSmartie", "coap-net-iu0to"), &fetcher).unwrap();
        assert_eq!(value, "no builds found");
    }

    #[test]
    fn reports_no_tests_when_total_is_zero() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/NZSmartie/coap-net-iu0to",
            body: r#"{"build": {"status": "success", "jobs": [
                {"name": "a", "status": "success", "testsCount": 0, "passedTestsCount": 0, "failedTestsCount": 0}
            ]}}"#,
        };
        let value = resolve_tests(&params("NZSmartie", "coap-net-iu0to"), &fetcher).unwrap();
        assert_eq!(value, "no tests");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_tests(&HashMap::new(), &Unused).is_err());
        assert!(resolve_tests(&params("NZSmartie", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_tests(&params("../etc", "coap-net-iu0to"), &Unused).is_err());
    }

    #[test]
    fn errors_when_a_job_is_missing_a_count_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/NZSmartie/coap-net-iu0to",
            body: r#"{"build": {"status": "success", "jobs": [{"name": "a", "status": "success"}]}}"#,
        };
        assert!(resolve_tests(&params("NZSmartie", "coap-net-iu0to"), &fetcher).is_err());
    }
}
