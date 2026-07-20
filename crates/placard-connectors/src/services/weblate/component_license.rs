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

pub(crate) fn resolve_component_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("weblate-component-license requires a data-project attribute")?;
    let component = params
        .get("component")
        .ok_or("weblate-component-license requires a data-component attribute")?;
    let project = validate_path_param("project", project)?;
    let component = validate_path_param("component", component)?;
    let server = resolve_server(params)?;

    let url = format!("{server}/api/components/{project}/{component}/");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "weblate response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("license")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "weblate response missing license".to_string())
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

    fn params(project: &str, component: &str) -> HashMap<String, String> {
        HashMap::from([
            ("project".to_string(), project.to_string()),
            ("component".to_string(), component.to_string()),
        ])
    }

    #[test]
    fn extracts_the_license_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/components/godot-engine/godot/",
            body: r#"{"license": "MIT"}"#,
        };
        let value = resolve_component_license(&params("godot-engine", "godot"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn uses_a_custom_server_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://example.com/api/components/godot-engine/godot/",
            body: r#"{"license": "GPL-3.0"}"#,
        };
        let mut p = params("godot-engine", "godot");
        p.insert("server".to_string(), "https://example.com".to_string());
        let value = resolve_component_license(&p, &fetcher).unwrap();
        assert_eq!(value, "GPL-3.0");
    }

    #[test]
    fn requires_project_and_component_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_component_license(&HashMap::new(), &Unused).is_err());
        assert!(resolve_component_license(&params("godot-engine", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_component_license(&params("../etc", "godot"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/components/godot-engine/godot/",
            body: r#"{"slug": "godot"}"#,
        };
        assert!(resolve_component_license(&params("godot-engine", "godot"), &fetcher).is_err());
    }
}
