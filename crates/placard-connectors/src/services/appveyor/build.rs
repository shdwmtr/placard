use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_build(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("appveyor-build requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("appveyor-build requires a data-repo attribute")?;
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

    let status = value
        .get("build.status")
        .ok_or("appveyor response missing build.status")?;
    status
        .as_text()
        .ok_or_else(|| "build.status was not a plain value".to_string())
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
    fn extracts_build_status_from_an_appveyor_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/gruntjs/grunt",
            body: r#"{"build": {"status": "success", "jobs": []}}"#,
        };
        let value = resolve_build(&params("gruntjs", "grunt"), &fetcher).unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn appends_branch_to_the_url_when_provided() {
        let mut p = params("gruntjs", "grunt");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/gruntjs/grunt/branch/master",
            body: r#"{"build": {"status": "failed", "jobs": []}}"#,
        };
        let value = resolve_build(&p, &fetcher).unwrap();
        assert_eq!(value, "failed");
    }

    #[test]
    fn reports_no_builds_found_when_build_key_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/gruntjs/grunt",
            body: r#"{}"#,
        };
        let value = resolve_build(&params("gruntjs", "grunt"), &fetcher).unwrap();
        assert_eq!(value, "no builds found");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_build(&HashMap::new(), &Unused).is_err());
        assert!(resolve_build(&params("gruntjs", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_build(&params("../etc", "grunt"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.appveyor.com/api/projects/gruntjs/grunt",
            body: r#"{"build": {"jobs": []}}"#,
        };
        assert!(resolve_build(&params("gruntjs", "grunt"), &fetcher).is_err());
    }
}
