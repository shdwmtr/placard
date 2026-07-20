use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_filename(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'filename' parameter must not be empty".to_string());
    }
    if value.contains("..")
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    {
        return Err("'filename' parameter contains disallowed characters".to_string());
    }
    Ok(value)
}

pub(crate) fn resolve_manifest(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-manifest requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-manifest requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };
    let filename = match params.get("filename") {
        Some(filename) => validate_filename(filename)?,
        None => "manifest.json",
    };
    let key = match params.get("key") {
        Some(key) if key.is_empty() => return Err("'key' parameter must not be empty".to_string()),
        Some(key) => key.as_str(),
        None => "version",
    };

    let url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{filename}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let field = value
        .get(key)
        .ok_or_else(|| format!("manifest response missing {key}"))?;
    match field {
        Value::Array(items) => {
            let parts: Result<Vec<String>, String> = items
                .iter()
                .map(|item| {
                    item.as_text()
                        .ok_or_else(|| format!("{key} array contained a non-scalar value"))
                })
                .collect();
            Ok(parts?.join(", "))
        }
        other => other
            .as_text()
            .ok_or_else(|| format!("{key} was not a plain value")),
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

    fn params(owner: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_version_field_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/sindresorhus/show-all-github-issues/HEAD/manifest.json",
            body: r#"{"name": "Show all GitHub issues", "version": "1.4.0"}"#,
        };
        let value =
            resolve_manifest(&params("sindresorhus", "show-all-github-issues"), &fetcher).unwrap();
        assert_eq!(value, "1.4.0");
    }

    #[test]
    fn extracts_an_arbitrary_key_when_requested() {
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/sindresorhus/show-all-github-issues/HEAD/manifest.json",
            body: r#"{"name": "Show all GitHub issues", "version": "1.4.0"}"#,
        };
        let mut p = params("sindresorhus", "show-all-github-issues");
        p.insert("key".to_string(), "name".to_string());
        let value = resolve_manifest(&p, &fetcher).unwrap();
        assert_eq!(value, "Show all GitHub issues");
    }

    #[test]
    fn joins_an_array_valued_key_with_commas() {
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/sindresorhus/show-all-github-issues/HEAD/manifest.json",
            body: r#"{"version": "1.0.0", "permissions": ["tabs", "storage"]}"#,
        };
        let mut p = params("sindresorhus", "show-all-github-issues");
        p.insert("key".to_string(), "permissions".to_string());
        let value = resolve_manifest(&p, &fetcher).unwrap();
        assert_eq!(value, "tabs, storage");
    }

    #[test]
    fn uses_the_given_branch_and_filename_when_provided() {
        struct BranchFetcher;
        impl Fetcher for BranchFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://raw.githubusercontent.com/sindresorhus/ext/master/extension/manifest.json"
                );
                Ok(br#"{"version": "2.0.0"}"#.to_vec())
            }
        }
        let mut p = params("sindresorhus", "ext");
        p.insert("branch".to_string(), "master".to_string());
        p.insert(
            "filename".to_string(),
            "extension/manifest.json".to_string(),
        );
        let value = resolve_manifest(&p, &BranchFetcher).unwrap();
        assert_eq!(value, "2.0.0");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_manifest(&HashMap::new(), &Unused).is_err());
        assert!(resolve_manifest(&params("sindresorhus", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_manifest(&params("../etc", "ext"), &Unused).is_err());
    }

    #[test]
    fn rejects_a_traversal_filename() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with a traversal filename")
            }
        }
        let mut p = params("sindresorhus", "ext");
        p.insert("filename".to_string(), "../../etc/passwd".to_string());
        assert!(resolve_manifest(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/sindresorhus/show-all-github-issues/HEAD/manifest.json",
            body: r#"{"name": "Show all GitHub issues"}"#,
        };
        assert!(
            resolve_manifest(&params("sindresorhus", "show-all-github-issues"), &fetcher).is_err()
        );
    }
}
