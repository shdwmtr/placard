use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_popularity(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("aur-popularity requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://aur.archlinux.org/rpc?v=5&type=info&arg={package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "aur response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let results = value.get("results").ok_or("aur response missing results")?;
    let Value::Array(results) = results else {
        return Err("aur response's results field was not an array".to_string());
    };
    let first = results.first().ok_or("aur package not found")?;
    let popularity = first
        .get("Popularity")
        .ok_or("aur result missing Popularity")?;
    popularity
        .as_text()
        .ok_or_else(|| "Popularity was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    const URL: &str = "https://aur.archlinux.org/rpc?v=5&type=info&arg=dropbox";

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_popularity_from_an_aur_shaped_response() {
        let fetcher = FakeFetcher(
            URL,
            r#"{"resultcount": 1, "results": [{"Popularity": 1.23}]}"#,
        );
        let value = resolve_popularity(&params("dropbox"), &fetcher).unwrap();
        assert_eq!(value, "1.23");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_popularity(&HashMap::new(), &Unused).is_err());
        assert!(resolve_popularity(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_popularity(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_popularity_is_missing() {
        let fetcher = FakeFetcher(URL, r#"{"resultcount": 1, "results": [{}]}"#);
        assert!(resolve_popularity(&params("dropbox"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_package_not_found() {
        let fetcher = FakeFetcher(
            "https://aur.archlinux.org/rpc?v=5&type=info&arg=nonexistent-pkg",
            r#"{"resultcount": 0, "results": []}"#,
        );
        assert!(resolve_popularity(&params("nonexistent-pkg"), &fetcher).is_err());
    }
}
