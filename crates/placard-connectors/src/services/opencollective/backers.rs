use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn count_backers(value: &Value) -> Result<usize, String> {
    let Value::Array(members) = value else {
        return Err("opencollective response was not a JSON array".to_string());
    };
    Ok(members
        .iter()
        .filter(|member| member.get("role").and_then(Value::as_text).as_deref() == Some("BACKER"))
        .count())
}

pub(crate) fn resolve_backers(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let collective = params
        .get("collective")
        .ok_or("opencollective-backers requires a data-collective attribute")?;
    let collective = validate_path_param("collective", collective)?;

    let url = format!("https://opencollective.com/{collective}/members/users.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "opencollective response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    Ok(count_backers(&value)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://opencollective.com/shields/members/users.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(collective: &str) -> HashMap<String, String> {
        HashMap::from([("collective".to_string(), collective.to_string())])
    }

    #[test]
    fn counts_individual_members_with_backer_role() {
        let fetcher = FakeFetcher(
            r#"[{"MemberId": 1, "type": "USER", "role": "BACKER"}, {"MemberId": 2, "type": "USER", "role": "BACKER"}]"#,
        );
        let value = resolve_backers(&params("shields"), &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn returns_zero_for_no_backers() {
        let fetcher = FakeFetcher("[]");
        let value = resolve_backers(&params("shields"), &fetcher).unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn requires_collective_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_backers(&HashMap::new(), &Unused).is_err());
        assert!(resolve_backers(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_backers(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_not_an_array() {
        let fetcher = FakeFetcher(r#"{"error": "not found"}"#);
        assert!(resolve_backers(&params("shields"), &fetcher).is_err());
    }
}
