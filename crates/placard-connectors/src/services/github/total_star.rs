use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_total_star(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("github-total-star requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;

    let url = format!("https://api.github.com/users/{user}/repos?per_page=100");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Value::Array(repos) = value else {
        return Err("github response was not a JSON array".to_string());
    };

    let total: f64 = repos
        .iter()
        .filter_map(|repo| repo.get("stargazers_count"))
        .map(|v| match v {
            Value::Number(n) => *n,
            _ => 0.0,
        })
        .sum();

    Ok(if total.fract() == 0.0 && total.abs() < 1e15 {
        format!("{}", total as i64)
    } else {
        total.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/users/chris48s/repos?per_page=100"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str) -> HashMap<String, String> {
        HashMap::from([("user".to_string(), user.to_string())])
    }

    #[test]
    fn sums_stargazers_count_across_repos() {
        let fetcher = FakeFetcher(
            r#"[{"name": "a", "stargazers_count": 10}, {"name": "b", "stargazers_count": 5}, {"name": "c", "stargazers_count": 0}]"#,
        );
        let value = resolve_total_star(&params("chris48s"), &fetcher).unwrap();
        assert_eq!(value, "15");
    }

    #[test]
    fn returns_zero_for_no_repos() {
        let fetcher = FakeFetcher("[]");
        let value = resolve_total_star(&params("chris48s"), &fetcher).unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn requires_user_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_total_star(&HashMap::new(), &Unused).is_err());
        assert!(resolve_total_star(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_total_star(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_not_an_array() {
        let fetcher = FakeFetcher(r#"{"message": "Not Found"}"#);
        assert!(resolve_total_star(&params("chris48s"), &fetcher).is_err());
    }
}
