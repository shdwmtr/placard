use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let library = params
        .get("library")
        .ok_or("ctan-version requires a data-library attribute")?;
    let library = validate_path_param("library", library)?;

    let url = format!("https://www.ctan.org/json/2.0/pkg/{library}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "ctan response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let number = value
        .get("version.number")
        .and_then(|v| v.as_text())
        .filter(|s| !s.is_empty());
    if let Some(number) = number {
        return Ok(number);
    }
    let date = value
        .get("version.date")
        .and_then(|v| v.as_text())
        .filter(|s| !s.is_empty());
    date.ok_or_else(|| "ctan response had no version number or date".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://www.ctan.org/json/2.0/pkg/tex");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(library: &str) -> HashMap<String, String> {
        HashMap::from([("library".to_string(), library.to_string())])
    }

    #[test]
    fn extracts_version_number_when_present() {
        let fetcher = FakeFetcher(r#"{"version": {"number": "3.14159265", "date": "2021-01-01"}}"#);
        let value = resolve_version(&params("tex"), &fetcher).unwrap();
        assert_eq!(value, "3.14159265");
    }

    #[test]
    fn falls_back_to_date_when_number_is_empty() {
        let fetcher = FakeFetcher(r#"{"version": {"number": "", "date": "2021-01-01"}}"#);
        let value = resolve_version(&params("tex"), &fetcher).unwrap();
        assert_eq!(value, "2021-01-01");
    }

    #[test]
    fn requires_library_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_both_number_and_date_are_empty() {
        let fetcher = FakeFetcher(r#"{"version": {"number": "", "date": ""}}"#);
        assert!(resolve_version(&params("tex"), &fetcher).is_err());
    }
}
