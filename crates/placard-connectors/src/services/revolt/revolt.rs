use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn resolve_base_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("revolt-api-url") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'revolt-api-url' parameter must not be empty".to_string());
            }
            if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
                return Err("'revolt-api-url' parameter must be an http(s) URL".to_string());
            }
            if trimmed
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
            {
                return Err("'revolt-api-url' parameter contains disallowed characters".to_string());
            }
            Ok(trimmed.to_string())
        }
        None => Ok("https://api.revolt.chat".to_string()),
    }
}

pub(crate) fn resolve_revolt(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let invite_id = params
        .get("invite-id")
        .ok_or("revolt requires a data-invite-id attribute")?;
    let invite_id = validate_path_param("invite-id", invite_id)?;
    let base_url = resolve_base_url(params)?;

    let url = format!("{base_url}/invites/{invite_id}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "revolt response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("member_count")
        .ok_or("revolt response missing member_count")?;
    count
        .as_text()
        .ok_or_else(|| "member_count was not a plain value".to_string())
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

    fn params(invite_id: &str) -> HashMap<String, String> {
        HashMap::from([("invite-id".to_string(), invite_id.to_string())])
    }

    #[test]
    fn extracts_the_member_count_using_the_default_api_url() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.revolt.chat/invites/01F7ZSBSFHQ8TA81725KQCSDDP",
            body: r#"{"member_count": 4200}"#,
        };
        let value = resolve_revolt(&params("01F7ZSBSFHQ8TA81725KQCSDDP"), &fetcher).unwrap();
        assert_eq!(value, "4200");
    }

    #[test]
    fn uses_a_custom_revolt_api_url_when_given() {
        let mut p = params("01F7ZSBSFHQ8TA81725KQCSDDP");
        p.insert(
            "revolt-api-url".to_string(),
            "https://self-hosted.example.com".to_string(),
        );
        let fetcher = FakeFetcher {
            expected_url: "https://self-hosted.example.com/invites/01F7ZSBSFHQ8TA81725KQCSDDP",
            body: r#"{"member_count": 12}"#,
        };
        let value = resolve_revolt(&p, &fetcher).unwrap();
        assert_eq!(value, "12");
    }

    #[test]
    fn requires_invite_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_revolt(&HashMap::new(), &Unused).is_err());
        assert!(resolve_revolt(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_revolt(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_member_count_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.revolt.chat/invites/01F7ZSBSFHQ8TA81725KQCSDDP",
            body: r#"{"server_id": "abc"}"#,
        };
        assert!(resolve_revolt(&params("01F7ZSBSFHQ8TA81725KQCSDDP"), &fetcher).is_err());
    }
}
