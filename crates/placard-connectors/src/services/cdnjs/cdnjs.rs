use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_cdnjs(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let library = params
        .get("library")
        .ok_or("cdnjs requires a data-library attribute")?;
    let library = validate_path_param("library", library)?;

    let url = format!("https://api.cdnjs.com/libraries/{library}?fields=version");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "cdnjs response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("cdnjs response missing version (library not found)")?;
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
            assert_eq!(url, "https://api.cdnjs.com/libraries/jquery?fields=version");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(library: &str) -> HashMap<String, String> {
        HashMap::from([("library".to_string(), library.to_string())])
    }

    #[test]
    fn extracts_version_from_a_cdnjs_shaped_response() {
        let fetcher = FakeFetcher(r#"{"version": "3.6.0"}"#);
        let value = resolve_cdnjs(&params("jquery"), &fetcher).unwrap();
        assert_eq!(value, "3.6.0");
    }

    #[test]
    fn requires_a_library_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_cdnjs(&HashMap::new(), &Unused).is_err());
        assert!(resolve_cdnjs(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_cdnjs(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_library_is_not_found() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_cdnjs(&params("jquery"), &fetcher).is_err());
    }
}
