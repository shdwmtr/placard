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

pub(crate) fn resolve_by_tier(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let collective = params
        .get("collective")
        .ok_or("opencollective-by-tier requires a data-collective attribute")?;
    let collective = validate_path_param("collective", collective)?;
    let tier_id = params
        .get("tier-id")
        .ok_or("opencollective-by-tier requires a data-tier-id attribute")?;
    let tier_id = validate_path_param("tier-id", tier_id)?;

    let url = format!("https://opencollective.com/{collective}/members/all.json?TierId={tier_id}");
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
            assert_eq!(
                url,
                "https://opencollective.com/shields/members/all.json?TierId=2988"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(collective: &str, tier_id: &str) -> HashMap<String, String> {
        HashMap::from([
            ("collective".to_string(), collective.to_string()),
            ("tier-id".to_string(), tier_id.to_string()),
        ])
    }

    #[test]
    fn counts_members_with_backer_role_for_the_tier() {
        let fetcher = FakeFetcher(
            r#"[{"MemberId": 1, "type": "USER", "role": "BACKER", "tier": "sponsor"}, {"MemberId": 2, "type": "USER", "role": "BACKER", "tier": "sponsor"}]"#,
        );
        let value = resolve_by_tier(&params("shields", "2988"), &fetcher).unwrap();
        assert_eq!(value, "2");
    }

    #[test]
    fn returns_zero_for_no_members_in_the_tier() {
        let fetcher = FakeFetcher("[]");
        let value = resolve_by_tier(&params("shields", "2988"), &fetcher).unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn requires_collective_and_tier_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_by_tier(&HashMap::new(), &Unused).is_err());
        assert!(resolve_by_tier(&params("shields", ""), &Unused).is_err());
        assert!(resolve_by_tier(&params("", "2988"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_by_tier(&params("../etc", "2988"), &Unused).is_err());
        assert!(resolve_by_tier(&params("shields", "1?x=2"), &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_not_an_array() {
        let fetcher = FakeFetcher(r#"{"error": "not found"}"#);
        assert!(resolve_by_tier(&params("shields", "2988"), &fetcher).is_err());
    }
}
