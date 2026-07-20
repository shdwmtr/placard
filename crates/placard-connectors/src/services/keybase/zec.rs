use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_zec(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let username = params
        .get("username")
        .ok_or("keybase-zec requires a data-username attribute")?;
    let username = validate_path_param("username", username)?;

    let url = format!(
        "https://keybase.io/_/api/1.0/user/lookup.json?usernames={username}&fields=cryptocurrency_addresses"
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

    let addresses = user
        .get("cryptocurrency_addresses.zcash")
        .ok_or("keybase response missing zcash addresses")?;
    let Value::Array(items) = addresses else {
        return Err("keybase zcash addresses was not an array".to_string());
    };
    let first = items.first().ok_or("no zcash addresses found")?;
    first
        .get("address")
        .ok_or("keybase zcash entry missing address")?
        .as_text()
        .ok_or_else(|| "zcash address was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://keybase.io/_/api/1.0/user/lookup.json?usernames=skyplabs&fields=cryptocurrency_addresses"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(username: &str) -> HashMap<String, String> {
        HashMap::from([("username".to_string(), username.to_string())])
    }

    #[test]
    fn extracts_the_first_zcash_address() {
        let fetcher = FakeFetcher(
            r#"{"status": {"code": 0}, "them": [{"cryptocurrency_addresses": {"zcash": [{"address": "z1abc"}]}}]}"#,
        );
        let value = resolve_zec(&params("skyplabs"), &fetcher).unwrap();
        assert_eq!(value, "z1abc");
    }

    #[test]
    fn requires_username_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid username")
            }
        }
        assert!(resolve_zec(&HashMap::new(), &Unused).is_err());
        assert!(resolve_zec(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid username")
            }
        }
        assert!(resolve_zec(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_zcash_addresses_are_found() {
        let fetcher = FakeFetcher(
            r#"{"status": {"code": 0}, "them": [{"cryptocurrency_addresses": {"zcash": []}}]}"#,
        );
        assert!(resolve_zec(&params("skyplabs"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_profile_is_missing() {
        let fetcher = FakeFetcher(r#"{"status": {"code": 0}, "them": []}"#);
        assert!(resolve_zec(&params("skyplabs"), &fetcher).is_err());
    }
}
