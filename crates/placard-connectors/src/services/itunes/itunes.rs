use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_itunes(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let bundle_id = params
        .get("bundle-id")
        .ok_or("itunes requires a data-bundle-id attribute")?;
    let bundle_id = validate_path_param("bundle-id", bundle_id)?;

    let url = format!("https://itunes.apple.com/lookup?id={bundle_id}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "itunes response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let json::Value::Array(results) = value
        .get("results")
        .ok_or("itunes response missing results")?
    else {
        return Err("itunes results was not an array".to_string());
    };
    let first = results.first().ok_or("itunes app not found")?;
    let version = first
        .get("version")
        .ok_or("itunes response missing version")?;
    version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://itunes.apple.com/lookup?id=803453959");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(bundle_id: &str) -> HashMap<String, String> {
        HashMap::from([("bundle-id".to_string(), bundle_id.to_string())])
    }

    #[test]
    fn extracts_version_from_the_first_result() {
        let fetcher = FakeFetcher(r#"{"resultCount": 1, "results": [{"version": "3.4.1"}]}"#);
        let value = resolve_itunes(&params("803453959"), &fetcher).unwrap();
        assert_eq!(value, "3.4.1");
    }

    #[test]
    fn errors_when_no_results_are_found() {
        let fetcher = FakeFetcher(r#"{"resultCount": 0, "results": []}"#);
        assert!(resolve_itunes(&params("803453959"), &fetcher).is_err());
    }

    #[test]
    fn requires_bundle_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_itunes(&HashMap::new(), &Unused).is_err());
        assert!(resolve_itunes(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_itunes(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_version_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"resultCount": 1, "results": [{"trackId": 1}]}"#);
        assert!(resolve_itunes(&params("803453959"), &fetcher).is_err());
    }
}
