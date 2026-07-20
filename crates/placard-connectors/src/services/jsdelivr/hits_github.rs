use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn period_word(period: &str) -> Result<&'static str, String> {
    match period {
        "hd" => Ok("day"),
        "hw" => Ok("week"),
        "hm" => Ok("month"),
        "hy" => Ok("year"),
        _ => Err("'period' parameter must be one of hd, hw, hm, hy".to_string()),
    }
}

pub(crate) fn resolve_hits_github(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let period = params
        .get("period")
        .ok_or("jsdelivr-hits-github requires a data-period attribute")?;
    let period_word = period_word(period)?;
    let owner = params
        .get("owner")
        .ok_or("jsdelivr-hits-github requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("jsdelivr-hits-github requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url =
        format!("https://data.jsdelivr.com/v1/package/gh/{owner}/{repo}/stats/date/{period_word}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "jsdelivr response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let total = value
        .get("total")
        .ok_or("jsdelivr response missing total")?;
    total
        .as_text()
        .ok_or_else(|| "total was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://data.jsdelivr.com/v1/package/gh/jquery/jquery/stats/date/month"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(period: &str, owner: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("period".to_string(), period.to_string()),
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_total_from_a_jsdelivr_shaped_response() {
        let fetcher = FakeFetcher(r#"{"total": 123456}"#);
        let value = resolve_hits_github(&params("hm", "jquery", "jquery"), &fetcher).unwrap();
        assert_eq!(value, "123456");
    }

    #[test]
    fn requires_period_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_hits_github(&HashMap::new(), &Unused).is_err());
        assert!(resolve_hits_github(&params("hm", "jquery", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_an_invalid_period() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid period")
            }
        }
        assert!(resolve_hits_github(&params("bogus", "jquery", "jquery"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_hits_github(&params("hm", "../etc", "jquery"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"day": "2024-01-01"}"#);
        assert!(resolve_hits_github(&params("hm", "jquery", "jquery"), &fetcher).is_err());
    }
}
