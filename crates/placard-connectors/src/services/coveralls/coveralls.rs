use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_vcs_type(value: &str) -> Result<&str, String> {
    match value {
        "github" | "bitbucket" | "gitlab" => Ok(value),
        other => Err(format!(
            "'vcs-type' parameter '{other}' is not one of github, bitbucket, gitlab"
        )),
    }
}

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

pub(crate) fn resolve_coveralls(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("coveralls requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("coveralls requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let vcs_type = match params.get("vcs-type") {
        Some(value) => validate_vcs_type(value)?,
        None => "github",
    };
    let branch = params.get("branch").map(String::as_str).unwrap_or("@");

    let url = format!(
        "https://coveralls.io/{vcs_type}/{user}/{repo}.json?branch={}",
        percent_encode(branch)
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "coveralls response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let covered_percent = value
        .get("covered_percent")
        .ok_or("coveralls response missing covered_percent")?;
    match covered_percent {
        json::Value::Number(n) => Ok(format!("{}%", n.round() as i64)),
        _ => Err("covered_percent was not a number".to_string()),
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
    fn extracts_and_rounds_covered_percent_with_default_vcs_and_branch() {
        let fetcher = FakeFetcher {
            expected_url: "https://coveralls.io/github/jekyll/jekyll.json?branch=%40",
            body: r#"{"covered_percent": 92.34}"#,
        };
        let value = resolve_coveralls(&params("jekyll", "jekyll"), &fetcher).unwrap();
        assert_eq!(value, "92%");
    }

    #[test]
    fn uses_custom_vcs_type_and_branch_when_provided() {
        let mut p = params("jekyll", "jekyll");
        p.insert("vcs-type".to_string(), "gitlab".to_string());
        p.insert("branch".to_string(), "main".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://coveralls.io/gitlab/jekyll/jekyll.json?branch=main",
            body: r#"{"covered_percent": 50}"#,
        };
        let value = resolve_coveralls(&p, &fetcher).unwrap();
        assert_eq!(value, "50%");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_coveralls(&HashMap::new(), &Unused).is_err());
        assert!(resolve_coveralls(&params("jekyll", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_and_bad_vcs_type() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_coveralls(&params("../etc", "jekyll"), &Unused).is_err());
        let mut p = params("jekyll", "jekyll");
        p.insert("vcs-type".to_string(), "svn".to_string());
        assert!(resolve_coveralls(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://coveralls.io/github/jekyll/jekyll.json?branch=%40",
            body: r#"{"other": 1}"#,
        };
        assert!(resolve_coveralls(&params("jekyll", "jekyll"), &fetcher).is_err());
    }
}
