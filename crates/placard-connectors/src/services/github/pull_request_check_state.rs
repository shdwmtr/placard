use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_pull_request_check_state(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-pull-request-check-state requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-pull-request-check-state requires a data-repo attribute")?;
    let number = params
        .get("number")
        .ok_or("github-pull-request-check-state requires a data-number attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let number = validate_path_param("number", number)?;

    let pulls_url = format!("https://api.github.com/repos/{owner}/{repo}/pulls/{number}");
    let bytes = fetcher.fetch(&pulls_url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let pull = json::parse(&text)?;
    let sha = pull
        .get("head.sha")
        .and_then(|v| v.as_text())
        .ok_or("github response missing head.sha")?;
    let sha = validate_path_param("sha", &sha)?;

    let status_url = format!("https://api.github.com/repos/{owner}/{repo}/commits/{sha}/status");
    let bytes = fetcher.fetch(&status_url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let status = json::parse(&text)?;
    let state = status.get("state").ok_or("github response missing state")?;
    state
        .as_text()
        .ok_or_else(|| "state was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeFetcher {
        pull_response: &'static str,
        status_response: &'static str,
        calls: AtomicUsize,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => {
                    assert_eq!(
                        url,
                        "https://api.github.com/repos/badges/shields/pulls/1110"
                    );
                    Ok(self.pull_response.as_bytes().to_vec())
                }
                1 => {
                    assert_eq!(
                        url,
                        "https://api.github.com/repos/badges/shields/commits/abc123/status"
                    );
                    Ok(self.status_response.as_bytes().to_vec())
                }
                _ => panic!("unexpected extra fetch"),
            }
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "badges".to_string()),
            ("repo".to_string(), "shields".to_string()),
            ("number".to_string(), "1110".to_string()),
        ])
    }

    #[test]
    fn extracts_the_combined_status_state() {
        let fetcher = FakeFetcher {
            pull_response: r#"{"head": {"sha": "abc123"}}"#,
            status_response: r#"{"state": "success", "statuses": []}"#,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_pull_request_check_state(&params(), &fetcher).unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn requires_owner_repo_and_number_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_pull_request_check_state(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("number".to_string(), String::new());
        assert!(resolve_pull_request_check_state(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params();
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_pull_request_check_state(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_state_field_is_missing() {
        let fetcher = FakeFetcher {
            pull_response: r#"{"head": {"sha": "abc123"}}"#,
            status_response: r#"{"statuses": []}"#,
            calls: AtomicUsize::new(0),
        };
        assert!(resolve_pull_request_check_state(&params(), &fetcher).is_err());
    }
}
