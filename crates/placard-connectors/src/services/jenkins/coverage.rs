use crate::Fetcher;
use crate::json;
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

pub(crate) fn resolve_coverage(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let job_url = params
        .get("job-url")
        .ok_or("jenkins-coverage requires a data-job-url attribute")?;
    let job_url = validate_job_url(job_url)?;

    let url =
        format!("{job_url}/lastCompletedBuild/coverage/api/json?tree=projectStatistics%5Bline%5D");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "jenkins response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let line = value
        .get("projectStatistics.line")
        .ok_or("jenkins response missing projectStatistics.line")?;
    let line = line
        .as_text()
        .ok_or_else(|| "projectStatistics.line was not a plain value".to_string())?;
    let numeric = line
        .strip_suffix('%')
        .ok_or_else(|| "projectStatistics.line was not a percentage".to_string())?;
    let coverage: f64 = numeric
        .parse()
        .map_err(|_| "projectStatistics.line was not a number".to_string())?;

    Ok(format!("{}%", coverage.round() as i64))
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
    fn extracts_line_coverage_rounded_to_a_whole_number() {
        let fetcher = FakeFetcher {
            expected_url: "https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master/lastCompletedBuild/coverage/api/json?tree=projectStatistics%5Bline%5D",
            body: r#"{"projectStatistics": {"line": "93.0%"}}"#,
        };
        let value = resolve_coverage(
            &params("https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "93%");
    }

    #[test]
    fn rounds_half_up() {
        let fetcher = FakeFetcher {
            expected_url: "https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master/lastCompletedBuild/coverage/api/json?tree=projectStatistics%5Bline%5D",
            body: r#"{"projectStatistics": {"line": "87.65%"}}"#,
        };
        let value = resolve_coverage(
            &params("https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "88%");
    }

    #[test]
    fn requires_job_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid job-url")
            }
        }
        assert!(resolve_coverage(&HashMap::new(), &Unused).is_err());
        assert!(resolve_coverage(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_a_non_http_job_url() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid job-url")
            }
        }
        assert!(resolve_coverage(&params("not-a-url"), &Unused).is_err());
    }

    #[test]
    fn errors_when_line_coverage_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master/lastCompletedBuild/coverage/api/json?tree=projectStatistics%5Bline%5D",
            body: r#"{"projectStatistics": {}}"#,
        };
        assert!(
            resolve_coverage(
                &params("https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master"),
                &fetcher
            )
            .is_err()
        );
    }

    #[test]
    fn errors_when_line_coverage_is_not_a_percentage() {
        let fetcher = FakeFetcher {
            expected_url: "https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master/lastCompletedBuild/coverage/api/json?tree=projectStatistics%5Bline%5D",
            body: r#"{"projectStatistics": {"line": "93.0"}}"#,
        };
        assert!(
            resolve_coverage(
                &params("https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master"),
                &fetcher
            )
            .is_err()
        );
    }
}
