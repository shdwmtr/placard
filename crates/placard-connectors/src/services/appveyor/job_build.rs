use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_job_build(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("appveyor-job-build requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("appveyor-job-build requires a data-repo attribute")?;
    let job = params
        .get("job")
        .ok_or("appveyor-job-build requires a data-job attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    if job.is_empty() {
        return Err("'job' parameter must not be empty".to_string());
    }

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

    let job_value = items
        .iter()
        .find(|item| item.get("name").and_then(Value::as_text).as_deref() == Some(job.as_str()))
        .ok_or_else(|| format!("job '{job}' not found"))?;

    let status = job_value.get("status").ok_or("job entry missing status")?;
    status
        .as_text()
        .ok_or_else(|| "job status was not a plain value".to_string())
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

    fn params(user: &str, repo: &str, job: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("job".to_string(), job.to_string()),
        ])
    }

    #[test]
    fn extracts_the_named_jobs_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/wpmgprostotema/voicetranscoder",
            body: r#"{"build": {"status": "success", "jobs": [
                {"name": "Linux", "status": "success", "testsCount": 1, "passedTestsCount": 1, "failedTestsCount": 0},
                {"name": "Windows", "status": "failed", "testsCount": 1, "passedTestsCount": 0, "failedTestsCount": 1}
            ]}}"#,
        };
        let value = resolve_job_build(
            &params("wpmgprostotema", "voicetranscoder", "Windows"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "failed");
    }

    #[test]
    fn appends_branch_to_the_url_when_provided() {
        let mut p = params("wpmgprostotema", "voicetranscoder", "Linux");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/wpmgprostotema/voicetranscoder/branch/master",
            body: r#"{"build": {"status": "success", "jobs": [{"name": "Linux", "status": "success"}]}}"#,
        };
        let value = resolve_job_build(&p, &fetcher).unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn reports_no_builds_found_when_build_key_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/wpmgprostotema/voicetranscoder",
            body: r#"{}"#,
        };
        let value = resolve_job_build(
            &params("wpmgprostotema", "voicetranscoder", "Linux"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "no builds found");
    }

    #[test]
    fn requires_user_repo_and_job_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_job_build(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_job_build(&params("wpmgprostotema", "voicetranscoder", ""), &Unused).is_err()
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
        assert!(resolve_job_build(&params("../etc", "voicetranscoder", "Linux"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_job_name_is_not_found() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/wpmgprostotema/voicetranscoder",
            body: r#"{"build": {"status": "success", "jobs": [{"name": "Linux", "status": "success"}]}}"#,
        };
        assert!(
            resolve_job_build(
                &params("wpmgprostotema", "voicetranscoder", "Windows"),
                &fetcher
            )
            .is_err()
        );
    }
}
