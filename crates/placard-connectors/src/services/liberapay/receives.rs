use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_receives(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let entity = params
        .get("entity")
        .ok_or("liberapay-receives requires a data-entity attribute")?;
    let entity = validate_path_param("entity", entity)?;

    let url = format!("https://liberapay.com/{entity}/public.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "liberapay response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    match value.get("receiving") {
        Some(Value::Null) | None => Err("no public receiving stats".to_string()),
        Some(receiving) => {
            let amount = receiving
                .get("amount")
                .ok_or("liberapay receiving entry missing amount")?
                .as_text()
                .ok_or_else(|| "receiving amount was not a plain value".to_string())?;
            let currency = receiving
                .get("currency")
                .ok_or("liberapay receiving entry missing currency")?
                .as_text()
                .ok_or_else(|| "receiving currency was not a plain value".to_string())?;
            Ok(format!("{amount} {currency}/week"))
        }
    }
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
    fn extracts_the_receiving_amount_and_currency() {
        let fetcher = FakeFetcher(
            r#"{"npatrons": 3, "giving": null, "receiving": {"amount": "250.00", "currency": "USD"}, "goal": null}"#,
        );
        let value = resolve_receives(&params("Changaco"), &fetcher).unwrap();
        assert_eq!(value, "250.00 USD/week");
    }

    #[test]
    fn requires_entity_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid entity")
            }
        }
        assert!(resolve_receives(&HashMap::new(), &Unused).is_err());
        assert!(resolve_receives(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid entity")
            }
        }
        assert!(resolve_receives(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_public_receiving_stats() {
        let fetcher =
            FakeFetcher(r#"{"npatrons": 0, "giving": null, "receiving": null, "goal": null}"#);
        assert!(resolve_receives(&params("Changaco"), &fetcher).is_err());
    }
}
