use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_implementation(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-implementation requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://pypi.org/pypi/{package}/json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "pypi response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let classifiers = value
        .get("info.classifiers")
        .ok_or("pypi response missing info.classifiers")?;
    let Value::Array(items) = classifiers else {
        return Err("pypi response 'info.classifiers' was not an array".to_string());
    };

    let texts: Vec<String> = items.iter().filter_map(|item| item.as_text()).collect();
    let prefix = "Programming Language :: Python :: Implementation :: ";
    let mut implementations: Vec<String> = texts
        .iter()
        .filter_map(|s| s.strip_prefix(prefix))
        .filter(|s| !s.is_empty() && !s.contains(char::is_whitespace))
        .map(|s| s.to_string())
        .collect();

    if implementations.is_empty() {
        implementations.push("cpython".to_string());
    }

    implementations.sort();
    Ok(implementations.join(" | "))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://pypi.org/pypi/requests/json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_and_sorts_implementations() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": [
                "Programming Language :: Python :: Implementation :: PyPy",
                "Programming Language :: Python :: Implementation :: CPython"
            ]}}"#,
        );
        let value = resolve_implementation(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "CPython | PyPy");
    }

    #[test]
    fn defaults_to_cpython_when_no_implementation_classifiers_are_present() {
        let fetcher =
            FakeFetcher(r#"{"info": {"classifiers": ["Programming Language :: Python :: 3"]}}"#);
        let value = resolve_implementation(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "cpython");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_implementation(&HashMap::new(), &Unused).is_err());
        assert!(resolve_implementation(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_implementation(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_classifiers_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"info": {}}"#);
        assert!(resolve_implementation(&params("requests"), &fetcher).is_err());
    }
}
