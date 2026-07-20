use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_pgp(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let username = params
        .get("username")
        .ok_or("keybase-pgp requires a data-username attribute")?;
    let username = validate_path_param("username", username)?;

    let url = format!(
        "https://keybase.io/_/api/1.0/user/lookup.json?usernames={username}&fields=public_keys"
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "keybase response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let them = value.get("them").ok_or("keybase response missing them")?;
    let Value::Array(them_items) = them else {
        return Err("keybase response 'them' was not an array".to_string());
    };
    let user = them_items.first().ok_or("keybase profile not found")?;

    let fingerprint = user
        .get("public_keys.primary.key_fingerprint")
        .ok_or("no key fingerprint found")?
        .as_text()
        .ok_or_else(|| "key fingerprint was not a plain value".to_string())?;

    let short = if fingerprint.len() > 16 {
        &fingerprint[fingerprint.len() - 16..]
    } else {
        &fingerprint[..]
    };
    Ok(short.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://keybase.io/_/api/1.0/user/lookup.json?usernames=skyplabs&fields=public_keys"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(username: &str) -> HashMap<String, String> {
        HashMap::from([("username".to_string(), username.to_string())])
    }

    #[test]
    fn extracts_the_last_16_chars_of_the_fingerprint_uppercased() {
        let fetcher = FakeFetcher(
            r#"{"status": {"code": 0}, "them": [{"public_keys": {"primary": {"key_fingerprint": "abcd1234abcd1234abcd1234abcd1234abcd1234"}}}]}"#,
        );
        let value = resolve_pgp(&params("skyplabs"), &fetcher).unwrap();
        assert_eq!(value, "ABCD1234ABCD1234");
    }

    #[test]
    fn requires_username_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid username")
            }
        }
        assert!(resolve_pgp(&HashMap::new(), &Unused).is_err());
        assert!(resolve_pgp(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid username")
            }
        }
        assert!(resolve_pgp(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_key_fingerprint_is_found() {
        let fetcher = FakeFetcher(r#"{"status": {"code": 0}, "them": [{"public_keys": {}}]}"#);
        assert!(resolve_pgp(&params("skyplabs"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_profile_is_missing() {
        let fetcher = FakeFetcher(r#"{"status": {"code": 0}, "them": []}"#);
        assert!(resolve_pgp(&params("skyplabs"), &fetcher).is_err());
    }
}
