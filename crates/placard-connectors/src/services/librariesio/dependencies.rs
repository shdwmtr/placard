use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_dependencies(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("librariesio-dependencies requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("librariesio-dependencies requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://libraries.io/api/github/{user}/{repo}/shields_dependencies");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "libraries.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let deprecated = match value.get("deprecated_count") {
        Some(Value::Number(n)) => *n as i64,
        _ => return Err("libraries.io response missing deprecated_count".to_string()),
    };
    let outdated = match value.get("outdated_count") {
        Some(Value::Number(n)) => *n as i64,
        _ => return Err("libraries.io response missing outdated_count".to_string()),
    };

    if deprecated > 0 {
        Ok(format!("{deprecated} deprecated"))
    } else if outdated > 0 {
        Ok(format!("{outdated} out of date"))
    } else {
        Ok("up to date".to_string())
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
    fn reports_up_to_date_when_no_deprecated_or_outdated() {
        let fetcher = FakeFetcher {
            expected_url: "https://libraries.io/api/github/phoenixframework/phoenix/shields_dependencies",
            body: r#"{"deprecated_count": 0, "outdated_count": 0}"#,
        };
        let value = resolve_dependencies(&params("phoenixframework", "phoenix"), &fetcher).unwrap();
        assert_eq!(value, "up to date");
    }

    #[test]
    fn reports_out_of_date_count_when_no_deprecated() {
        let fetcher = FakeFetcher {
            expected_url: "https://libraries.io/api/github/phoenixframework/phoenix/shields_dependencies",
            body: r#"{"deprecated_count": 0, "outdated_count": 17}"#,
        };
        let value = resolve_dependencies(&params("phoenixframework", "phoenix"), &fetcher).unwrap();
        assert_eq!(value, "17 out of date");
    }

    #[test]
    fn reports_deprecated_count_taking_priority_over_outdated() {
        let fetcher = FakeFetcher {
            expected_url: "https://libraries.io/api/github/phoenixframework/phoenix/shields_dependencies",
            body: r#"{"deprecated_count": 2, "outdated_count": 17}"#,
        };
        let value = resolve_dependencies(&params("phoenixframework", "phoenix"), &fetcher).unwrap();
        assert_eq!(value, "2 deprecated");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_dependencies(&HashMap::new(), &Unused).is_err());
        assert!(resolve_dependencies(&params("phoenixframework", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_dependencies(&params("../etc", "phoenix"), &Unused).is_err());
    }

    #[test]
    fn errors_when_a_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://libraries.io/api/github/phoenixframework/phoenix/shields_dependencies",
            body: r#"{"deprecated_count": 0}"#,
        };
        assert!(resolve_dependencies(&params("phoenixframework", "phoenix"), &fetcher).is_err());
    }
}
