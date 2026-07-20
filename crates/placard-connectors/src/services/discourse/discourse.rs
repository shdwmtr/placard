use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_variant(value: &str) -> Result<&str, String> {
    match value {
        "topics" | "users" | "posts" | "likes" | "status" => Ok(value),
        other => Err(format!(
            "'variant' parameter '{other}' is not one of topics, users, posts, likes, status"
        )),
    }
}

fn validate_server(value: &str) -> Result<&str, String> {
    let trimmed = value.trim_end_matches('/');
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
    Ok(trimmed)
}

fn singular(variant: &str) -> &str {
    &variant[..variant.len() - 1]
}

pub(crate) fn resolve_discourse(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let server = params
        .get("server")
        .ok_or("discourse requires a data-server attribute")?;
    let server = validate_server(server)?;
    let variant = params
        .get("variant")
        .ok_or("discourse requires a data-variant attribute")?;
    let variant = validate_variant(variant)?;

    let url = format!("{server}/site/statistics.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "discourse response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    if variant == "status" {
        return Ok("online".to_string());
    }

    let singular_key = format!("{}_count", singular(variant));
    let plural_key = format!("{variant}_count");
    let stat = value
        .get(&singular_key)
        .or_else(|| value.get(&plural_key))
        .ok_or_else(|| format!("discourse response missing {singular_key}/{plural_key}"))?;
    stat.as_text()
        .ok_or_else(|| "discourse stat was not a plain value".to_string())
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

    fn params(server: &str, variant: &str) -> HashMap<String, String> {
        HashMap::from([
            ("server".to_string(), server.to_string()),
            ("variant".to_string(), variant.to_string()),
        ])
    }

    #[test]
    fn extracts_the_singular_count_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://meta.discourse.org/site/statistics.json",
            body: r#"{"topic_count": 4200, "user_count": 900}"#,
        };
        let value =
            resolve_discourse(&params("https://meta.discourse.org", "topics"), &fetcher).unwrap();
        assert_eq!(value, "4200");
    }

    #[test]
    fn falls_back_to_the_plural_count_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://meta.discourse.org/site/statistics.json",
            body: r#"{"topics_count": 4200}"#,
        };
        let value =
            resolve_discourse(&params("https://meta.discourse.org", "topics"), &fetcher).unwrap();
        assert_eq!(value, "4200");
    }

    #[test]
    fn returns_online_for_the_status_variant() {
        let fetcher = FakeFetcher {
            expected_url: "https://meta.discourse.org/site/statistics.json",
            body: r#"{"topic_count": 4200}"#,
        };
        let value =
            resolve_discourse(&params("https://meta.discourse.org", "status"), &fetcher).unwrap();
        assert_eq!(value, "online");
    }

    #[test]
    fn requires_server_and_variant_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_discourse(&HashMap::new(), &Unused).is_err());
        assert!(resolve_discourse(&params("", "topics"), &Unused).is_err());
    }

    #[test]
    fn rejects_a_non_http_server_and_a_bad_variant() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_discourse(&params("not-a-url", "topics"), &Unused).is_err());
        assert!(
            resolve_discourse(&params("https://meta.discourse.org", "comments"), &Unused).is_err()
        );
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://meta.discourse.org/site/statistics.json",
            body: r#"{"other": 1}"#,
        };
        assert!(
            resolve_discourse(&params("https://meta.discourse.org", "topics"), &fetcher).is_err()
        );
    }
}
