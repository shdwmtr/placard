use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_job_url(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'job-url' parameter must not be empty".to_string());
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err("'job-url' parameter must be an absolute http(s) URL".to_string());
    }
    if value.chars().any(|c| c.is_whitespace()) {
        return Err("'job-url' parameter must not contain whitespace".to_string());
    }
    Ok(value.trim_end_matches('/'))
}

fn field_as_i64(item: &Value, field: &str) -> Result<i64, String> {
    match item.get(field) {
        Some(Value::Number(n)) => Ok(*n as i64),
        _ => Err(format!("test result entry missing {field}")),
    }
}

pub(crate) fn resolve_tests(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let job_url = params
        .get("job-url")
        .ok_or("jenkins-tests requires a data-job-url attribute")?;
    let job_url = validate_job_url(job_url)?;

    let url = format!(
        "{job_url}/lastCompletedBuild/api/json?tree=actions%5BfailCount%2CskipCount%2CtotalCount%5D"
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "jenkins response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let actions = value
        .get("actions")
        .ok_or("jenkins response missing actions")?;
    let Value::Array(items) = actions else {
        return Err("actions was not an array".to_string());
    };

    let Some(entry) = items.iter().find(|item| item.get("failCount").is_some()) else {
        return Ok("no tests found".to_string());
    };

    let total = field_as_i64(entry, "totalCount")?;
    let failed = field_as_i64(entry, "failCount")?;
    let skipped = field_as_i64(entry, "skipCount")?;
    let passed = total - failed - skipped;

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

    fn params(job_url: &str) -> HashMap<String, String> {
        HashMap::from([("job-url".to_string(), job_url.to_string())])
    }

    #[test]
    fn summarizes_test_counts_from_the_matching_action() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/lastCompletedBuild/api/json?tree=actions%5BfailCount%2CskipCount%2CtotalCount%5D",
            body: r#"{"actions":[{"_class":"hudson.model.ParametersAction"},{"_class":"hudson.tasks.junit.TestResultAction","failCount":15,"skipCount":0,"totalCount":753},{}]}"#,
        };
        let value =
            resolve_tests(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "738 passed, 15 failed");
    }

    #[test]
    fn includes_skipped_when_present() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/lastCompletedBuild/api/json?tree=actions%5BfailCount%2CskipCount%2CtotalCount%5D",
            body: r#"{"actions":[{"failCount":1,"skipCount":2,"totalCount":10}]}"#,
        };
        let value =
            resolve_tests(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "7 passed, 1 failed, 2 skipped");
    }

    #[test]
    fn reports_no_tests_found_when_no_action_has_a_fail_count() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/orbit/job/orbit-shell/lastCompletedBuild/api/json?tree=actions%5BfailCount%2CskipCount%2CtotalCount%5D",
            body: r#"{"actions":[{"_class":"hudson.model.ParametersAction"},{}]}"#,
        };
        let value = resolve_tests(
            &params("https://ci.eclipse.org/orbit/job/orbit-shell"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "no tests found");
    }

    #[test]
    fn reports_no_tests_when_total_is_zero() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/lastCompletedBuild/api/json?tree=actions%5BfailCount%2CskipCount%2CtotalCount%5D",
            body: r#"{"actions":[{"failCount":0,"skipCount":0,"totalCount":0}]}"#,
        };
        let value =
            resolve_tests(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "no tests");
    }

    #[test]
    fn requires_job_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid job-url")
            }
        }
        assert!(resolve_tests(&HashMap::new(), &Unused).is_err());
        assert!(resolve_tests(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_a_non_http_job_url() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid job-url")
            }
        }
        assert!(resolve_tests(&params("not-a-url"), &Unused).is_err());
    }

    #[test]
    fn errors_when_actions_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/lastCompletedBuild/api/json?tree=actions%5BfailCount%2CskipCount%2CtotalCount%5D",
            body: r#"{}"#,
        };
        assert!(resolve_tests(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).is_err());
    }
}
