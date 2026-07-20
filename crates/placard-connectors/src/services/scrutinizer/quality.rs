use crate::Fetcher;
use crate::json::{self, Value};
use crate::services::validate_path_param;
use std::collections::HashMap;

fn obj_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

pub(crate) fn resolve_quality(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let vcs = params
        .get("vcs")
        .ok_or("scrutinizer-quality requires a data-vcs attribute")?;
    let user = params
        .get("user")
        .ok_or("scrutinizer-quality requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("scrutinizer-quality requires a data-repo attribute")?;
    let vcs = validate_path_param("vcs", vcs)?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = match params.get("branch") {
        Some(b) if !b.is_empty() => Some(validate_path_param("branch", b)?.to_string()),
        _ => None,
    };

    let url = format!("https://scrutinizer-ci.com/api/repositories/{vcs}/{user}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "scrutinizer response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;

    let branch_name = match branch {
        Some(b) => b,
        None => doc
            .get("default_branch")
            .and_then(Value::as_text)
            .ok_or("scrutinizer response missing default_branch")?,
    };

    let applications = doc
        .get("applications")
        .ok_or("scrutinizer response missing applications")?;
    let branch_info = obj_get(applications, &branch_name)
        .ok_or_else(|| format!("no quality info for branch '{branch_name}'"))?;
    let index = obj_get(branch_info, "index").ok_or("metrics missing for branch")?;
    let embedded = obj_get(index, "_embedded").ok_or("scrutinizer response missing _embedded")?;
    let project = obj_get(embedded, "project").ok_or("scrutinizer response missing project")?;
    let metric_values =
        obj_get(project, "metric_values").ok_or("scrutinizer response missing metric_values")?;
    let raw = obj_get(metric_values, "scrutinizer.quality")
        .ok_or("scrutinizer response missing scrutinizer.quality")?;
    let score = match raw {
        Value::Number(n) => *n,
        _ => return Err("scrutinizer.quality was not a number".to_string()),
    };

    let rounded = (score * 100.0).round() / 100.0;
    Ok(format!("{rounded}"))
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

    fn params(vcs: &str, user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("vcs".to_string(), vcs.to_string()),
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_quality_score_for_the_default_branch() {
        let fetcher = FakeFetcher {
            expected_url: "https://scrutinizer-ci.com/api/repositories/g/filp/whoops",
            body: r#"{"default_branch": "master", "applications": {"master": {"index": {"_embedded": {"project": {"metric_values": {"scrutinizer.quality": 8.1234}}}}}}}"#,
        };
        let value = resolve_quality(&params("g", "filp", "whoops"), &fetcher).unwrap();
        assert_eq!(value, "8.12");
    }

    #[test]
    fn requires_vcs_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_quality(&HashMap::new(), &Unused).is_err());
        assert!(resolve_quality(&params("g", "filp", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_quality(&params("g", "../etc", "whoops"), &Unused).is_err());
    }

    #[test]
    fn errors_when_metrics_are_missing_for_the_branch() {
        let fetcher = FakeFetcher {
            expected_url: "https://scrutinizer-ci.com/api/repositories/g/filp/whoops",
            body: r#"{"default_branch": "master", "applications": {"master": {}}}"#,
        };
        assert!(resolve_quality(&params("g", "filp", "whoops"), &fetcher).is_err());
    }
}
