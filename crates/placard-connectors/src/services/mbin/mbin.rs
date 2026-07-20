use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_mbin(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let magazine = params
        .get("magazine")
        .ok_or("mbin requires a data-magazine attribute")?;
    let mut parts = magazine.splitn(2, '@');
    let mag = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or("'magazine' parameter must not be empty")?;
    let host = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or("'magazine' parameter must be in the form magazine@server")?;
    let mag = validate_path_param("magazine", mag)?;
    let host = validate_path_param("magazine", host)?;

    let url = format!("https://{host}/api/magazine/name/{mag}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "mbin response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let members = value
        .get("subscriptionsCount")
        .ok_or("mbin response missing subscriptionsCount")?;
    members
        .as_text()
        .ok_or_else(|| "subscriptionsCount was not a plain value".to_string())
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

    fn params(magazine: &str) -> HashMap<String, String> {
        HashMap::from([("magazine".to_string(), magazine.to_string())])
    }

    #[test]
    fn extracts_subscriptions_count() {
        let fetcher = FakeFetcher {
            expected_url: "https://kbin.earth/api/magazine/name/kbinEarth",
            body: r#"{"subscriptionsCount": 314}"#,
        };
        let value = resolve_mbin(&params("kbinEarth@kbin.earth"), &fetcher).unwrap();
        assert_eq!(value, "314");
    }

    #[test]
    fn requires_a_magazine_at_server_shape() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid magazine")
            }
        }
        assert!(resolve_mbin(&HashMap::new(), &Unused).is_err());
        assert!(resolve_mbin(&params("noatsign"), &Unused).is_err());
        assert!(resolve_mbin(&params("mag@"), &Unused).is_err());
        assert!(resolve_mbin(&params("@server"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid magazine")
            }
        }
        assert!(resolve_mbin(&params("../etc@kbin.earth"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://kbin.earth/api/magazine/name/kbinEarth",
            body: r#"{}"#,
        };
        assert!(resolve_mbin(&params("kbinEarth@kbin.earth"), &fetcher).is_err());
    }
}
