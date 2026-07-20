use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_site(value: &str) -> Result<&str, String> {
    match value {
        "github" | "gitlab" | "bitbucket" | "sourcehut" | "codeberg" => Ok(value),
        other => Err(format!(
            "'site' parameter '{other}' is not one of github, gitlab, bitbucket, sourcehut, codeberg"
        )),
    }
}

pub(crate) fn resolve_repo(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let site = params
        .get("site")
        .ok_or("deps-rs-repo requires a data-site attribute")?;
    let site = validate_site(site)?;
    let user = params
        .get("user")
        .ok_or("deps-rs-repo requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = params
        .get("repo")
        .ok_or("deps-rs-repo requires a data-repo attribute")?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://deps.rs/repo/{site}/{user}/{repo}/shield.json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "deps.rs response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let message = value
        .get("message")
        .ok_or("deps.rs response missing message")?;
    message
        .as_text()
        .ok_or_else(|| "message was not a plain value".to_string())
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

    fn params(site: &str, user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("site".to_string(), site.to_string()),
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_message_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://deps.rs/repo/github/dtolnay/syn/shield.json",
            body: r#"{"message": "1 of 5 outdated"}"#,
        };
        let value = resolve_repo(&params("github", "dtolnay", "syn"), &fetcher).unwrap();
        assert_eq!(value, "1 of 5 outdated");
    }

    #[test]
    fn requires_site_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_repo(&HashMap::new(), &Unused).is_err());
        assert!(resolve_repo(&params("github", "dtolnay", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_and_bad_site() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_repo(&params("github", "../etc", "syn"), &Unused).is_err());
        assert!(resolve_repo(&params("svn", "dtolnay", "syn"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://deps.rs/repo/github/dtolnay/syn/shield.json",
            body: r#"{"other": 1}"#,
        };
        assert!(resolve_repo(&params("github", "dtolnay", "syn"), &fetcher).is_err());
    }
}
