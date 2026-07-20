use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_posts(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let actor = params
        .get("actor")
        .ok_or("bluesky-posts requires a data-actor attribute")?;
    let actor = validate_path_param("actor", actor)?;

    let url = format!("https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor={actor}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "bluesky response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let posts = value
        .get("postsCount")
        .ok_or("bluesky response missing postsCount")?;
    posts
        .as_text()
        .ok_or_else(|| "postsCount was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor=chitvs.bsky.social"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(actor: &str) -> HashMap<String, String> {
        HashMap::from([("actor".to_string(), actor.to_string())])
    }

    #[test]
    fn extracts_posts_count_from_a_bluesky_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"did":"did:plc:abc","handle":"chitvs.bsky.social","followersCount":1234,"postsCount":56}"#,
        );
        let value = resolve_posts(&params("chitvs.bsky.social"), &fetcher).unwrap();
        assert_eq!(value, "56");
    }

    #[test]
    fn requires_actor_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_posts(&HashMap::new(), &Unused).is_err());
        assert!(resolve_posts(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_posts(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"did":"did:plc:abc","handle":"chitvs.bsky.social"}"#);
        assert!(resolve_posts(&params("chitvs.bsky.social"), &fetcher).is_err());
    }
}
