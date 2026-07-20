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

pub(crate) fn resolve_last_commit(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("bitbucket-last-commit requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("bitbucket-last-commit requires a data-repo attribute")?;
    let branch = params
        .get("branch")
        .ok_or("bitbucket-last-commit requires a data-branch attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = validate_path_param("branch", branch)?;

    let mut url = format!(
        "https://bitbucket.org/api/2.0/repositories/{user}/{repo}/commits/{branch}?pagelen=1&fields=values.date"
    );
    if let Some(path) = params.get("path") {
        url.push_str(&format!("&path={}", percent_encode(path)));
    }

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
    let commit = items.first().ok_or("no commits found")?;
    let date = commit
        .get("date")
        .ok_or("bitbucket commit entry missing date")?;
    date.as_text()
        .ok_or_else(|| "date was not a plain value".to_string())
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

    #[test]
    fn extracts_the_date_of_the_latest_commit() {
        let fetcher = FakeFetcher {
            expected_url: "https://bitbucket.org/api/2.0/repositories/shields-io/test-repo/commits/main?pagelen=1&fields=values.date",
            body: r#"{"values": [{"date": "2024-05-01T12:00:00+00:00"}]}"#,
        };
        let value =
            resolve_last_commit(&params("shields-io", "test-repo", "main"), &fetcher).unwrap();
        assert_eq!(value, "2024-05-01T12:00:00+00:00");
    }

    #[test]
    fn appends_percent_encoded_path_to_the_url_when_provided() {
        let mut p = params("shields-io", "test-repo", "main");
        p.insert("path".to_string(), "docs/README.md".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://bitbucket.org/api/2.0/repositories/shields-io/test-repo/commits/main?pagelen=1&fields=values.date&path=docs%2FREADME.md",
            body: r#"{"values": [{"date": "2024-05-01T12:00:00+00:00"}]}"#,
        };
        let value = resolve_last_commit(&p, &fetcher).unwrap();
        assert_eq!(value, "2024-05-01T12:00:00+00:00");
    }

    #[test]
    fn requires_user_repo_and_branch_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_last_commit(&HashMap::new(), &Unused).is_err());
        assert!(resolve_last_commit(&params("shields-io", "test-repo", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_last_commit(&params("../etc", "test-repo", "main"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_commits() {
        let fetcher = FakeFetcher {
            expected_url: "https://bitbucket.org/api/2.0/repositories/shields-io/test-repo/commits/main?pagelen=1&fields=values.date",
            body: r#"{"values": []}"#,
        };
        assert!(resolve_last_commit(&params("shields-io", "test-repo", "main"), &fetcher).is_err());
    }
}
