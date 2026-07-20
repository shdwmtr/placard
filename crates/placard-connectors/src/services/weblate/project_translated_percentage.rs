use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
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

pub(crate) fn resolve_project_translated_percentage(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("weblate-project-translated-percentage requires a data-project attribute")?;
    let project = validate_path_param("project", project)?;
    let server = resolve_server(params)?;

    let url = format!("{server}/api/projects/{project}/statistics/");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "weblate response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let translated_percent = value
        .get("translated_percent")
        .ok_or("weblate response missing translated_percent")?;
    match translated_percent {
        Value::Number(n) => Ok(format!("{}%", n.round() as i64)),
        _ => Err("translated_percent was not a number".to_string()),
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

    fn params(project: &str) -> HashMap<String, String> {
        HashMap::from([("project".to_string(), project.to_string())])
    }

    #[test]
    fn extracts_and_rounds_the_translated_percent_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/projects/godot-engine/statistics/",
            body: r#"{"translated_percent": 87.6}"#,
        };
        let value =
            resolve_project_translated_percentage(&params("godot-engine"), &fetcher).unwrap();
        assert_eq!(value, "88%");
    }

    #[test]
    fn uses_a_custom_server_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://example.com/api/projects/godot-engine/statistics/",
            body: r#"{"translated_percent": 50}"#,
        };
        let mut p = params("godot-engine");
        p.insert("server".to_string(), "https://example.com".to_string());
        let value = resolve_project_translated_percentage(&p, &fetcher).unwrap();
        assert_eq!(value, "50%");
    }

    #[test]
    fn requires_a_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project")
            }
        }
        assert!(resolve_project_translated_percentage(&HashMap::new(), &Unused).is_err());
        assert!(resolve_project_translated_percentage(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_project_translated_percentage(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/projects/godot-engine/statistics/",
            body: r#"{"total": 100}"#,
        };
        assert!(resolve_project_translated_percentage(&params("godot-engine"), &fetcher).is_err());
    }
}
