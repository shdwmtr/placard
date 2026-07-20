use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_score(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("dub-score requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://code.dlang.org/api/packages/{package}/stats");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "dub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let score = match value.get("score") {
        Some(Value::Number(n)) => *n,
        _ => return Err("dub response missing score".to_string()),
    };
    Ok(format!("{score:.1}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://code.dlang.org/api/packages/vibe-d/stats");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn formats_the_score_to_one_decimal() {
        let fetcher = FakeFetcher(r#"{"score": 3.14159}"#);
        let value = resolve_score(&params("vibe-d"), &fetcher).unwrap();
        assert_eq!(value, "3.1");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_score(&HashMap::new(), &Unused).is_err());
        assert!(resolve_score(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_score(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"downloads": {"total": 1}}"#);
        assert!(resolve_score(&params("vibe-d"), &fetcher).is_err());
    }
}
