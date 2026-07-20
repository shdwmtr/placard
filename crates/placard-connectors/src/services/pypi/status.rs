use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-status requires a data-package attribute")?;
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

    let mut matches: Vec<String> = items
        .iter()
        .filter_map(|item| item.as_text())
        .filter_map(|s| parse_status_classifier(&s))
        .collect();
    matches.sort();

    let status = matches
        .last()
        .and_then(|s| s.split(" - ").nth(1))
        .unwrap_or("Unknown");
    let status = if status.eq_ignore_ascii_case("production/stable") {
        "stable"
    } else {
        status
    };

    Ok(status.to_lowercase())
}

fn parse_status_classifier(s: &str) -> Option<String> {
    let rest = s.strip_prefix("Development Status :: ")?;
    let mut parts = rest.splitn(2, " - ");
    let digits = parts.next()?;
    let word = parts.next()?;
    if !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && !word.is_empty()
        && !word.contains(char::is_whitespace)
    {
        Some(rest.to_string())
    } else {
        None
    }
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
    fn extracts_and_normalizes_production_stable() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": [
                "Development Status :: 3 - Alpha",
                "Development Status :: 5 - Production/Stable"
            ]}}"#,
        );
        let value = resolve_status(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "stable");
    }

    #[test]
    fn picks_the_highest_status_when_multiple_are_present() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": [
                "Development Status :: 2 - Pre-Alpha",
                "Development Status :: 4 - Beta"
            ]}}"#,
        );
        let value = resolve_status(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "beta");
    }

    #[test]
    fn defaults_to_unknown_when_no_status_classifier_is_present() {
        let fetcher =
            FakeFetcher(r#"{"info": {"classifiers": ["Programming Language :: Python :: 3"]}}"#);
        let value = resolve_status(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "unknown");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_status(&HashMap::new(), &Unused).is_err());
        assert!(resolve_status(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_status(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_classifiers_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"info": {}}"#);
        assert!(resolve_status(&params("requests"), &fetcher).is_err());
    }
}
