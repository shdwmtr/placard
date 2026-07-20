use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn resolve_server(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("server") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'server' parameter must not be empty".to_string());
            }
            if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
                return Err("'server' parameter must be an http(s) URL".to_string());
            }
            if trimmed
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
            {
                return Err("'server' parameter contains disallowed characters".to_string());
            }
            Ok(trimmed.to_string())
        }
        None => Ok("https://hosted.weblate.org".to_string()),
    }
}

fn statistic_field(statistic: &str) -> Result<&'static str, String> {
    match statistic {
        "translations" => Ok("translated"),
        "suggestions" => Ok("suggested"),
        "uploads" => Ok("uploaded"),
        "comments" => Ok("commented"),
        "languages" => Ok("languages"),
        other => Err(format!(
            "'statistic' parameter '{other}' is not one of translations, suggestions, languages, uploads, comments"
        )),
    }
}

pub(crate) fn resolve_user_statistic(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let statistic = params
        .get("statistic")
        .ok_or("weblate-user-statistic requires a data-statistic attribute")?;
    let field = statistic_field(statistic)?;
    let user = params
        .get("user")
        .ok_or("weblate-user-statistic requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;
    let server = resolve_server(params)?;

    let url = format!("{server}/api/users/{user}/statistics/");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "weblate response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get(field)
        .and_then(|v| v.as_text())
        .ok_or_else(|| format!("weblate response missing {field}"))
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

    fn params(statistic: &str, user: &str) -> HashMap<String, String> {
        HashMap::from([
            ("statistic".to_string(), statistic.to_string()),
            ("user".to_string(), user.to_string()),
        ])
    }

    #[test]
    fn extracts_the_field_matching_the_statistic() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/users/nijel/statistics/",
            body: r#"{"translated": 5000, "suggested": 12, "uploaded": 3, "commented": 7, "languages": 4}"#,
        };
        let value = resolve_user_statistic(&params("translations", "nijel"), &fetcher).unwrap();
        assert_eq!(value, "5000");
    }

    #[test]
    fn maps_uploads_to_the_uploaded_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/users/nijel/statistics/",
            body: r#"{"translated": 5000, "suggested": 12, "uploaded": 3, "commented": 7, "languages": 4}"#,
        };
        let value = resolve_user_statistic(&params("uploads", "nijel"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn uses_a_custom_server_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://example.com/api/users/nijel/statistics/",
            body: r#"{"translated": 1, "suggested": 2, "uploaded": 3, "commented": 4, "languages": 5}"#,
        };
        let mut p = params("languages", "nijel");
        p.insert("server".to_string(), "https://example.com".to_string());
        let value = resolve_user_statistic(&p, &fetcher).unwrap();
        assert_eq!(value, "5");
    }

    #[test]
    fn requires_valid_statistic_and_user_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_user_statistic(&HashMap::new(), &Unused).is_err());
        assert!(resolve_user_statistic(&params("bogus", "nijel"), &Unused).is_err());
        assert!(resolve_user_statistic(&params("translations", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_user_statistic(&params("translations", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/users/nijel/statistics/",
            body: r#"{"suggested": 12}"#,
        };
        assert!(resolve_user_statistic(&params("translations", "nijel"), &fetcher).is_err());
    }
}
