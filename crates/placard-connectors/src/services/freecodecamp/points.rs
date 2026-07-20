use super::super::validate_path_param;
use crate::json::Value;
use crate::{Fetcher, json};
use std::collections::HashMap;

pub(crate) fn resolve_points(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let username = params
        .get("username")
        .ok_or("freecodecamp-points requires a data-username attribute")?;
    let username = validate_path_param("username", username)?;

    let url = format!("https://api.freecodecamp.org/users/get-public-profile?username={username}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "freecodecamp response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let Some(Value::Object(user_fields)) = value.get("entities.user") else {
        return Err("freecodecamp response missing entities.user".to_string());
    };
    let user = user_fields
        .iter()
        .find(|(k, _)| k == username)
        .map(|(_, v)| v)
        .ok_or("profile not found")?;
    let points = user
        .get("points")
        .ok_or("freecodecamp response missing points")?;
    match points {
        Value::Null => Err("private".to_string()),
        other => other
            .as_text()
            .ok_or_else(|| "points was not a plain value".to_string()),
    }
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

    fn params(username: &str) -> HashMap<String, String> {
        HashMap::from([("username".to_string(), username.to_string())])
    }

    #[test]
    fn extracts_the_points_for_the_given_username() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.freecodecamp.org/users/get-public-profile?username=qapaloma",
            body: r#"{"entities": {"user": {"qapaloma": {"points": 1234}}}}"#,
        };
        let value = resolve_points(&params("qapaloma"), &fetcher).unwrap();
        assert_eq!(value, "1234");
    }

    #[test]
    fn requires_a_username_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid username")
            }
        }
        assert!(resolve_points(&HashMap::new(), &Unused).is_err());
        assert!(resolve_points(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_username() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid username")
            }
        }
        assert!(resolve_points(&params("a/b"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_profile_is_private() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.freecodecamp.org/users/get-public-profile?username=qapaloma",
            body: r#"{"entities": {"user": {"qapaloma": {"points": null}}}}"#,
        };
        assert!(resolve_points(&params("qapaloma"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_profile_is_not_found() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.freecodecamp.org/users/get-public-profile?username=qapaloma",
            body: r#"{"entities": {"user": {}}}"#,
        };
        assert!(resolve_points(&params("qapaloma"), &fetcher).is_err());
    }
}
