use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_followers(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("github-followers requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;

    let url = format!("https://api.github.com/users/{user}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("followers")
        .ok_or("github response missing followers")?;
    count
        .as_text()
        .ok_or_else(|| "followers was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.github.com/users/espadrine");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str) -> HashMap<String, String> {
        HashMap::from([("user".to_string(), user.to_string())])
    }

    #[test]
    fn extracts_followers_from_a_github_user_response() {
        let fetcher = FakeFetcher(r#"{"login": "espadrine", "followers": 1234}"#);
        let value = resolve_followers(&params("espadrine"), &fetcher).unwrap();
        assert_eq!(value, "1234");
    }

    #[test]
    fn requires_user_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_followers(&HashMap::new(), &Unused).is_err());
        assert!(resolve_followers(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_followers(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"login": "espadrine"}"#);
        assert!(resolve_followers(&params("espadrine"), &fetcher).is_err());
    }
}
