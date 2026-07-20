use super::validate_path_param;
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
    let owner = params
        .get("owner")
        .ok_or("github-last-commit requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-last-commit requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let display_field = match params.get("display-timestamp").map(String::as_str) {
        None | Some("author") => "author",
        Some("committer") => "committer",
        Some(other) => {
            return Err(format!(
                "'display-timestamp' parameter '{other}' is not one of author, committer"
            ));
        }
    };

    let mut url = format!("https://api.github.com/repos/{owner}/{repo}/commits?per_page=1");
    if let Some(branch) = params.get("branch") {
        if branch.is_empty() {
            return Err("'branch' parameter must not be empty".to_string());
        }
        url.push_str(&format!("&sha={}", percent_encode(branch)));
    }
    if let Some(path) = params.get("path") {
        if path.is_empty() {
            return Err("'path' parameter must not be empty".to_string());
        }
        url.push_str(&format!("&path={}", percent_encode(path)));
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(items) = &value else {
        return Err("github response was not an array".to_string());
    };
    let commit = items.first().ok_or("no commits found")?;
    let date_path = format!("commit.{display_field}.date");
    let date = commit
        .get(&date_path)
        .ok_or_else(|| format!("commit response missing {date_path}"))?;
    date.as_text()
        .ok_or_else(|| "commit date was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: String,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    fn commit_body(author_date: &str, committer_date: &str) -> String {
        format!(
            r#"[{{"commit": {{"author": {{"date": "{author_date}"}}, "committer": {{"date": "{committer_date}"}}}}}}]"#
        )
    }

    #[test]
    fn extracts_the_author_date_of_the_latest_commit_by_default() {
        let body = commit_body("2021-01-01T00:00:00Z", "2021-01-02T00:00:00Z");
        let fetcher = FakeFetcher {
            expected_url: "https://api.github.com/repos/google/skia/commits?per_page=1",
            body,
        };
        let value = resolve_last_commit(&params("google", "skia"), &fetcher).unwrap();
        assert_eq!(value, "2021-01-01T00:00:00Z");
    }

    #[test]
    fn extracts_the_committer_date_when_requested() {
        let body = commit_body("2021-01-01T00:00:00Z", "2021-01-02T00:00:00Z");
        let fetcher = FakeFetcher {
            expected_url: "https://api.github.com/repos/google/skia/commits?per_page=1",
            body,
        };
        let mut p = params("google", "skia");
        p.insert("display-timestamp".to_string(), "committer".to_string());
        let value = resolve_last_commit(&p, &fetcher).unwrap();
        assert_eq!(value, "2021-01-02T00:00:00Z");
    }

    #[test]
    fn appends_branch_and_percent_encoded_path_to_the_url_when_provided() {
        let body = commit_body("2021-01-01T00:00:00Z", "2021-01-02T00:00:00Z");
        let fetcher = FakeFetcher {
            expected_url: "https://api.github.com/repos/google/skia/commits?per_page=1&sha=infra%2Fconfig&path=docs%2FREADME.md",
            body,
        };
        let mut p = params("google", "skia");
        p.insert("branch".to_string(), "infra/config".to_string());
        p.insert("path".to_string(), "docs/README.md".to_string());
        let value = resolve_last_commit(&p, &fetcher).unwrap();
        assert_eq!(value, "2021-01-01T00:00:00Z");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_last_commit(&HashMap::new(), &Unused).is_err());
        assert!(resolve_last_commit(&params("google", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_last_commit(&params("../etc", "skia"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_commits() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.github.com/repos/google/skia/commits?per_page=1",
            body: "[]".to_string(),
        };
        assert!(resolve_last_commit(&params("google", "skia"), &fetcher).is_err());
    }
}
