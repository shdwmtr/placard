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

pub(crate) fn resolve_entities(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let kind = params
        .get("type")
        .ok_or("weblate-entities requires a data-type attribute")?;
    if !matches!(
        kind.as_str(),
        "components" | "projects" | "users" | "languages"
    ) {
        return Err(
            "'type' parameter must be one of components, projects, users, languages".to_string(),
        );
    }
    let server = resolve_server(params)?;

    let url = format!("{server}/api/{kind}/");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "weblate response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("count")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "weblate response missing count".to_string())
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

    fn params(kind: &str) -> HashMap<String, String> {
        HashMap::from([("type".to_string(), kind.to_string())])
    }

    #[test]
    fn extracts_the_count_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/projects/",
            body: r#"{"count": 1234}"#,
        };
        let value = resolve_entities(&params("projects"), &fetcher).unwrap();
        assert_eq!(value, "1234");
    }

    #[test]
    fn uses_a_custom_server_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://example.com/api/users/",
            body: r#"{"count": 42}"#,
        };
        let mut p = params("users");
        p.insert("server".to_string(), "https://example.com".to_string());
        let value = resolve_entities(&p, &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_a_valid_type_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid type")
            }
        }
        assert!(resolve_entities(&HashMap::new(), &Unused).is_err());
        assert!(resolve_entities(&params("repositories"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://hosted.weblate.org/api/languages/",
            body: r#"{"results": []}"#,
        };
        assert!(resolve_entities(&params("languages"), &fetcher).is_err());
    }
}
