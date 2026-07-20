use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_gives(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let entity = params
        .get("entity")
        .ok_or("liberapay-gives requires a data-entity attribute")?;
    let entity = validate_path_param("entity", entity)?;

    let url = format!("https://liberapay.com/{entity}/public.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "liberapay response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    match value.get("giving") {
        Some(Value::Null) | None => Err("no public giving stats".to_string()),
        Some(giving) => {
            let amount = giving
                .get("amount")
                .ok_or("liberapay giving entry missing amount")?
                .as_text()
                .ok_or_else(|| "giving amount was not a plain value".to_string())?;
            let currency = giving
                .get("currency")
                .ok_or("liberapay giving entry missing currency")?
                .as_text()
                .ok_or_else(|| "giving currency was not a plain value".to_string())?;
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
    fn extracts_the_giving_amount_and_currency() {
        let fetcher = FakeFetcher(
            r#"{"npatrons": 3, "giving": {"amount": "10.00", "currency": "EUR"}, "receiving": null, "goal": null}"#,
        );
        let value = resolve_gives(&params("Changaco"), &fetcher).unwrap();
        assert_eq!(value, "10.00 EUR/week");
    }

    #[test]
    fn requires_entity_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid entity")
            }
        }
        assert!(resolve_gives(&HashMap::new(), &Unused).is_err());
        assert!(resolve_gives(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid entity")
            }
        }
        assert!(resolve_gives(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_public_giving_stats() {
        let fetcher =
            FakeFetcher(r#"{"npatrons": 0, "giving": null, "receiving": null, "goal": null}"#);
        assert!(resolve_gives(&params("Changaco"), &fetcher).is_err());
    }
}
