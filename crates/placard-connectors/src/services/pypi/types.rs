use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_types(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-types requires a data-package attribute")?;
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

    let is_typed = items
        .iter()
        .any(|item| item.as_text().as_deref() == Some("Typing :: Typed"));
    let is_stubs_only = items
        .iter()
        .any(|item| item.as_text().as_deref() == Some("Typing :: Stubs Only"));

    Ok(if is_typed {
        "typed".to_string()
    } else if is_stubs_only {
        "stubs".to_string()
    } else {
        "untyped".to_string()
    })
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
    fn reports_typed_when_typed_classifier_is_present() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": ["Programming Language :: Python", "Typing :: Typed"]}}"#,
        );
        let value = resolve_types(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "typed");
    }

    #[test]
    fn reports_stubs_when_stubs_only_classifier_is_present() {
        let fetcher = FakeFetcher(r#"{"info": {"classifiers": ["Typing :: Stubs Only"]}}"#);
        let value = resolve_types(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "stubs");
    }

    #[test]
    fn reports_untyped_when_neither_classifier_is_present() {
        let fetcher =
            FakeFetcher(r#"{"info": {"classifiers": ["Programming Language :: Python"]}}"#);
        let value = resolve_types(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "untyped");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_types(&HashMap::new(), &Unused).is_err());
        assert!(resolve_types(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_types(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_classifiers_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"info": {}}"#);
        assert!(resolve_types(&params("requests"), &fetcher).is_err());
    }
}
