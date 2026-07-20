use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_formula_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let formula = params
        .get("formula")
        .ok_or("homebrew-formula-version requires a data-formula attribute")?;
    let formula = validate_path_param("formula", formula)?;

    let url = format!("https://formulae.brew.sh/api/formula/{formula}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "homebrew response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let stable = value
        .get("versions.stable")
        .ok_or("homebrew response missing versions.stable")?;
    stable
        .as_text()
        .ok_or_else(|| "versions.stable was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://formulae.brew.sh/api/formula/cake.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(formula: &str) -> HashMap<String, String> {
        HashMap::from([("formula".to_string(), formula.to_string())])
    }

    #[test]
    fn extracts_stable_version_from_a_homebrew_formula_shaped_response() {
        let fetcher =
            FakeFetcher(r#"{"name": "cake", "versions": {"stable": "1.2.3", "head": "HEAD"}}"#);
        let value = resolve_formula_version(&params("cake"), &fetcher).unwrap();
        assert_eq!(value, "1.2.3");
    }

    #[test]
    fn requires_formula_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_formula_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_formula_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_formula_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "cake"}"#);
        assert!(resolve_formula_version(&params("cake"), &fetcher).is_err());
    }
}
