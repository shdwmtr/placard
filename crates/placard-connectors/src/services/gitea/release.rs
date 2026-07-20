use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn resolve_base_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("gitea-url") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'gitea-url' parameter must not be empty".to_string());
            }
            if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
                return Err("'gitea-url' parameter must be an http(s) URL".to_string());
            }
            if trimmed
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
            {
                return Err("'gitea-url' parameter contains disallowed characters".to_string());
            }
            Ok(trimmed.to_string())
        }
        None => Ok("https://gitea.com".to_string()),
    }
}

/// Only the default `date` sort (the order releases come back from Gitea)
/// is supported -- picking the "latest" release by semver comparison
/// across every release on the project is out of scope for a plain
/// field-extraction preset.
pub(crate) fn resolve_release(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("gitea-release requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("gitea-release requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let base_url = resolve_base_url(params)?;

    match params.get("sort").map(String::as_str) {
        None | Some("date") => {}
        Some("semver") => {
            return Err("gitea-release: data-sort=\"semver\" is not supported".to_string());
        }
        Some(other) => {
            return Err(format!(
                "gitea-release: unsupported data-sort value '{other}'"
            ));
        }
    }

    let display_field = match params.get("display-name").map(String::as_str) {
        None | Some("tag") => "tag_name",
        Some("release") => "name",
        Some(other) => {
            return Err(format!(
                "gitea-release: unsupported data-display-name value '{other}'"
            ));
        }
    };
    let include_prereleases = params.contains_key("include-prereleases");

    let url = format!("{base_url}/api/v1/repos/{user}/{repo}/releases");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gitea response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(releases) = value else {
        return Err("gitea response was not an array".to_string());
    };
    if releases.is_empty() {
        return Err("no releases found".to_string());
    }

    let chosen = if !include_prereleases {
        releases
            .iter()
            .find(|release| release.get("prerelease") != Some(&Value::Bool(true)))
            .or_else(|| releases.first())
    } else {
        releases.first()
    };
    let chosen = chosen.ok_or("no releases found")?;

    chosen
        .get(display_field)
        .and_then(Value::as_text)
        .ok_or_else(|| format!("gitea release entry missing {display_field}"))
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
    fn extracts_the_tag_name_of_the_first_stable_release_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea/releases",
            body: r#"[{"name": "v1.2 pre", "tag_name": "v1.2.0-rc1", "prerelease": true}, {"name": "Release 1.1", "tag_name": "v1.1.0", "prerelease": false}]"#,
        };
        let value = resolve_release(&params("gitea", "tea"), &fetcher).unwrap();
        assert_eq!(value, "v1.1.0");
    }

    #[test]
    fn includes_prereleases_when_requested() {
        let mut p = params("gitea", "tea");
        p.insert("include-prereleases".to_string(), "".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea/releases",
            body: r#"[{"name": "v1.2 pre", "tag_name": "v1.2.0-rc1", "prerelease": true}]"#,
        };
        let value = resolve_release(&p, &fetcher).unwrap();
        assert_eq!(value, "v1.2.0-rc1");
    }

    #[test]
    fn returns_the_name_field_when_display_name_is_release() {
        let mut p = params("gitea", "tea");
        p.insert("display-name".to_string(), "release".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea/releases",
            body: r#"[{"name": "Release 1.2", "tag_name": "v1.2.0", "prerelease": false}]"#,
        };
        let value = resolve_release(&p, &fetcher).unwrap();
        assert_eq!(value, "Release 1.2");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_release(&HashMap::new(), &Unused).is_err());
        assert!(resolve_release(&params("gitea", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_release(&params("../etc", "tea"), &Unused).is_err());
    }

    #[test]
    fn rejects_unsupported_semver_sort() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch when sort is unsupported")
            }
        }
        let mut p = params("gitea", "tea");
        p.insert("sort".to_string(), "semver".to_string());
        assert!(resolve_release(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_no_releases_exist() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitea.com/api/v1/repos/gitea/tea/releases",
            body: "[]",
        };
        assert!(resolve_release(&params("gitea", "tea"), &fetcher).is_err());
    }
}
