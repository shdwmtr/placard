use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub(crate) fn resolve_lemmy(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let community = params
        .get("community")
        .ok_or("lemmy requires a data-community attribute")?;
    let mut parts = community.splitn(2, '@');
    let name = parts.next().unwrap_or("");
    let host = parts
        .next()
        .ok_or("lemmy requires a data-community in the form name@host")?;
    let name = validate_path_param("community", name)?;
    let host = validate_path_param("community", host)?;

    let url = format!(
        "https://{host}/api/v3/community?name={}",
        percent_encode(&format!("{name}@{host}"))
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "lemmy response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let subscribers = value
        .get("community_view.counts.subscribers")
        .ok_or("lemmy response missing community_view.counts.subscribers")?;
    subscribers
        .as_text()
        .ok_or_else(|| "subscribers was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://lemmy.ml/api/v3/community?name=asklemmy%40lemmy.ml"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(community: &str) -> HashMap<String, String> {
        HashMap::from([("community".to_string(), community.to_string())])
    }

    #[test]
    fn extracts_subscribers_from_a_lemmy_shaped_response() {
        let fetcher = FakeFetcher(r#"{"community_view": {"counts": {"subscribers": 12483}}}"#);
        let value = resolve_lemmy(&params("asklemmy@lemmy.ml"), &fetcher).unwrap();
        assert_eq!(value, "12483");
    }

    #[test]
    fn requires_community_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_lemmy(&HashMap::new(), &Unused).is_err());
        assert!(resolve_lemmy(&params(""), &Unused).is_err());
    }

    #[test]
    fn requires_the_name_at_host_shape() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with a malformed community")
            }
        }
        assert!(resolve_lemmy(&params("asklemmy"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_lemmy(&params("../etc@lemmy.ml"), &Unused).is_err());
        assert!(resolve_lemmy(&params("asklemmy@../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_subscribers_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"community_view": {}}"#);
        assert!(resolve_lemmy(&params("asklemmy@lemmy.ml"), &fetcher).is_err());
    }
}
