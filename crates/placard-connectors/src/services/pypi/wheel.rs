use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_wheel(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-wheel requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://pypi.org/pypi/{package}/json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "pypi response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let urls = value.get("urls").ok_or("pypi response missing urls")?;
    let Value::Array(items) = urls else {
        return Err("pypi response 'urls' was not an array".to_string());
    };

    let has_wheel = items.iter().any(|item| {
        matches!(
            item.get("packagetype").and_then(|v| v.as_text()).as_deref(),
            Some("wheel") | Some("bdist_wheel")
        )
    });

    Ok(if has_wheel {
        "yes".to_string()
    } else {
        "no".to_string()
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
    fn reports_yes_when_a_wheel_is_present() {
        let fetcher =
            FakeFetcher(r#"{"urls": [{"packagetype": "sdist"}, {"packagetype": "bdist_wheel"}]}"#);
        let value = resolve_wheel(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "yes");
    }

    #[test]
    fn reports_no_when_no_wheel_is_present() {
        let fetcher = FakeFetcher(r#"{"urls": [{"packagetype": "sdist"}]}"#);
        let value = resolve_wheel(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "no");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_wheel(&HashMap::new(), &Unused).is_err());
        assert!(resolve_wheel(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_wheel(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_urls_field_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_wheel(&params("requests"), &fetcher).is_err());
    }
}
