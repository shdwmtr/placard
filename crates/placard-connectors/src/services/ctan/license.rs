use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let library = params
        .get("library")
        .ok_or("ctan-license requires a data-library attribute")?;
    let library = validate_path_param("library", library)?;

    let url = format!("https://www.ctan.org/json/2.0/pkg/{library}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "ctan response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let license = value
        .get("license")
        .ok_or("ctan response missing license")?;
    let Value::Array(items) = license else {
        return Err("license was not an array".to_string());
    };
    let mut names: Vec<String> = items.iter().filter_map(Value::as_text).collect();
    if names.is_empty() {
        return Err("license array was empty".to_string());
    }
    names.sort();
    Ok(names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://www.ctan.org/json/2.0/pkg/novel");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(library: &str) -> HashMap<String, String> {
        HashMap::from([("library".to_string(), library.to_string())])
    }

    #[test]
    fn extracts_sorted_license_list_from_a_ctan_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"license": ["lppl1.3", "gpl"], "version": {"number": "1.0", "date": "2020-01-01"}}"#,
        );
        let value = resolve_license(&params("novel"), &fetcher).unwrap();
        assert_eq!(value, "gpl, lppl1.3");
    }

    #[test]
    fn requires_library_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_license(&HashMap::new(), &Unused).is_err());
        assert!(resolve_license(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_license(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing_or_empty() {
        let fetcher = FakeFetcher(r#"{"version": {"number": "1.0", "date": ""}}"#);
        assert!(resolve_license(&params("novel"), &fetcher).is_err());

        let fetcher_empty =
            FakeFetcher(r#"{"license": [], "version": {"number": "1.0", "date": ""}}"#);
        assert!(resolve_license(&params("novel"), &fetcher_empty).is_err());
    }
}
