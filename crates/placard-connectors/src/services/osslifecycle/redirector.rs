use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn extract_status(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("osslifecycle=")?;
    let start = idx + "osslifecycle=".len();
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(rest[..end].to_ascii_lowercase())
}

/// Shields serves this as a redirect to its `osslifecycle` badge with
/// `file_url` computed from the repo's `OSSMETADATA` file; this preset
/// resolves the same underlying value directly.
pub(crate) fn resolve_redirector(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("osslifecycle-redirector requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("osslifecycle-redirector requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = params
        .get("branch")
        .map(String::as_str)
        .filter(|b| !b.is_empty())
        .unwrap_or("HEAD");
    let branch = validate_path_param("branch", branch)?;

    let url = format!("https://raw.githubusercontent.com/{user}/{repo}/{branch}/OSSMETADATA");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "osslifecycle response was not valid UTF-8".to_string())?;
    extract_status(&text).ok_or_else(|| "metadata in unexpected format".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/Netflix/aws-autoscaling/HEAD/OSSMETADATA"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn builds_the_raw_url_from_user_and_repo_and_extracts_status() {
        let fetcher = FakeFetcher("osslifecycle=maintenance\n");
        let value = resolve_redirector(&params("Netflix", "aws-autoscaling"), &fetcher).unwrap();
        assert_eq!(value, "maintenance");
    }

    #[test]
    fn uses_the_branch_param_when_given() {
        struct FakeFetcherBranch;
        impl Fetcher for FakeFetcherBranch {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://raw.githubusercontent.com/Netflix/aws-autoscaling/main/OSSMETADATA"
                );
                Ok(b"osslifecycle=active".to_vec())
            }
        }
        let mut p = params("Netflix", "aws-autoscaling");
        p.insert("branch".to_string(), "main".to_string());
        let value = resolve_redirector(&p, &FakeFetcherBranch).unwrap();
        assert_eq!(value, "active");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_redirector(&HashMap::new(), &Unused).is_err());
        assert!(resolve_redirector(&params("Netflix", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_redirector(&params("../etc", "aws-autoscaling"), &Unused).is_err());
    }

    #[test]
    fn errors_when_metadata_is_missing() {
        let fetcher = FakeFetcher("nothing to see here");
        assert!(resolve_redirector(&params("Netflix", "aws-autoscaling"), &fetcher).is_err());
    }
}
