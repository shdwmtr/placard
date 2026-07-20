use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_goal(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let entity = params
        .get("entity")
        .ok_or("liberapay-goal requires a data-entity attribute")?;
    let entity = validate_path_param("entity", entity)?;

    let url = format!("https://liberapay.com/{entity}/public.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "liberapay response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let goal_amount = match value.get("goal") {
        Some(Value::Null) | None => return Err("no public goals".to_string()),
        Some(goal) => goal
            .get("amount")
            .ok_or("liberapay goal entry missing amount")?
            .as_text()
            .ok_or_else(|| "goal amount was not a plain value".to_string())?
            .parse::<f64>()
            .map_err(|_| "goal amount was not a number".to_string())?,
    };

    let receiving_amount = match value.get("receiving") {
        Some(Value::Null) | None => return Ok("0%".to_string()),
        Some(receiving) => receiving
            .get("amount")
            .ok_or("liberapay receiving entry missing amount")?
            .as_text()
            .ok_or_else(|| "receiving amount was not a plain value".to_string())?
            .parse::<f64>()
            .map_err(|_| "receiving amount was not a number".to_string())?,
    };

    if goal_amount == 0.0 {
        return Err("liberapay goal amount was zero".to_string());
    }

    let percent_achieved = (receiving_amount / goal_amount * 100.0).round() as i64;
    Ok(format!("{percent_achieved}%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://liberapay.com/Changaco/public.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(entity: &str) -> HashMap<String, String> {
        HashMap::from([("entity".to_string(), entity.to_string())])
    }

    #[test]
    fn computes_the_percentage_achieved() {
        let fetcher = FakeFetcher(
            r#"{"npatrons": 3, "giving": null, "receiving": {"amount": "50.00", "currency": "EUR"}, "goal": {"amount": "200.00"}}"#,
        );
        let value = resolve_goal(&params("Changaco"), &fetcher).unwrap();
        assert_eq!(value, "25%");
    }

    #[test]
    fn returns_zero_percent_when_not_receiving_anything() {
        let fetcher = FakeFetcher(
            r#"{"npatrons": 0, "giving": null, "receiving": null, "goal": {"amount": "200.00"}}"#,
        );
        let value = resolve_goal(&params("Changaco"), &fetcher).unwrap();
        assert_eq!(value, "0%");
    }

    #[test]
    fn requires_entity_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid entity")
            }
        }
        assert!(resolve_goal(&HashMap::new(), &Unused).is_err());
        assert!(resolve_goal(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid entity")
            }
        }
        assert!(resolve_goal(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_public_goals() {
        let fetcher =
            FakeFetcher(r#"{"npatrons": 0, "giving": null, "receiving": null, "goal": null}"#);
        assert!(resolve_goal(&params("Changaco"), &fetcher).is_err());
    }
}
