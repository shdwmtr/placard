use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_patrons(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let entity = params
        .get("entity")
        .ok_or("liberapay-patrons requires a data-entity attribute")?;
    let entity = validate_path_param("entity", entity)?;

    let url = format!("https://liberapay.com/{entity}/public.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "liberapay response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    value
        .get("npatrons")
        .ok_or("liberapay response missing npatrons")?
        .as_text()
        .ok_or_else(|| "npatrons was not a plain value".to_string())
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
    fn extracts_the_patron_count() {
        let fetcher =
            FakeFetcher(r#"{"npatrons": 42, "giving": null, "receiving": null, "goal": null}"#);
        let value = resolve_patrons(&params("Changaco"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_entity_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid entity")
            }
        }
        assert!(resolve_patrons(&HashMap::new(), &Unused).is_err());
        assert!(resolve_patrons(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid entity")
            }
        }
        assert!(resolve_patrons(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_npatrons_is_missing() {
        let fetcher = FakeFetcher(r#"{"giving": null, "receiving": null, "goal": null}"#);
        assert!(resolve_patrons(&params("Changaco"), &fetcher).is_err());
    }
}
