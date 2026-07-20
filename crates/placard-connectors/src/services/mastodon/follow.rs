use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

const DEFAULT_DOMAIN: &str = "mastodon.social";

fn strip_scheme(domain: &str) -> &str {
    domain
        .strip_prefix("https://")
        .or_else(|| domain.strip_prefix("http://"))
        .unwrap_or(domain)
}

pub(crate) fn resolve_follow(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let id = params
        .get("id")
        .ok_or("mastodon-follow requires a data-id attribute")?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err("'id' parameter must be a numeric account id".to_string());
    }

    let domain = match params.get("domain") {
        Some(v) if !v.is_empty() => strip_scheme(v),
        _ => DEFAULT_DOMAIN,
    };
    let domain = validate_path_param("domain", domain)?;

    let url = format!("https://{domain}/api/v1/accounts/{id}/");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "mastodon response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let followers = value
        .get("followers_count")
        .ok_or("mastodon response missing followers_count")?;
    followers
        .as_text()
        .ok_or_else(|| "followers_count was not a plain value".to_string())
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

    fn params(id: &str) -> HashMap<String, String> {
        HashMap::from([("id".to_string(), id.to_string())])
    }

    #[test]
    fn extracts_followers_count_using_the_default_domain() {
        let fetcher = FakeFetcher {
            expected_url: "https://mastodon.social/api/v1/accounts/26471/",
            body: r#"{"username": "shdwmtr", "followers_count": 42}"#,
        };
        let value = resolve_follow(&params("26471"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn uses_a_custom_domain_and_strips_the_scheme() {
        let mut p = params("1");
        p.insert("domain".to_string(), "https://fosstodon.org".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://fosstodon.org/api/v1/accounts/1/",
            body: r#"{"username": "x", "followers_count": 7}"#,
        };
        let value = resolve_follow(&p, &fetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn requires_a_numeric_id() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid id")
            }
        }
        assert!(resolve_follow(&HashMap::new(), &Unused).is_err());
        assert!(resolve_follow(&params("not-a-number"), &Unused).is_err());
        assert!(resolve_follow(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_domain_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid domain")
            }
        }
        let mut p = params("1");
        p.insert("domain".to_string(), "evil.com/../etc".to_string());
        assert!(resolve_follow(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://mastodon.social/api/v1/accounts/1/",
            body: r#"{"username": "x"}"#,
        };
        assert!(resolve_follow(&params("1"), &fetcher).is_err());
    }
}
