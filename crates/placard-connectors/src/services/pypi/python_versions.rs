use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_python_versions(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-python-versions requires a data-package attribute")?;
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
    let prefix = "Programming Language :: Python :: ";

    let mut versions: Vec<String> = texts
        .iter()
        .filter_map(|s| s.strip_prefix(prefix))
        .filter(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .map(|s| s.to_string())
        .collect();

    if versions.is_empty() {
        versions = texts
            .iter()
            .filter_map(|s| s.strip_prefix(prefix))
            .filter_map(|rest| rest.strip_suffix(" :: Only"))
            .filter(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
            .map(|s| s.to_string())
            .collect();
    }

    let mut version_set: Vec<String> = Vec::new();
    for v in &versions {
        if !version_set.contains(v) {
            version_set.push(v.clone());
        }
    }

    for major in ["2", "3"] {
        let major_prefix = format!("{major}.");
        if versions
            .iter()
            .any(|v| v.starts_with(major_prefix.as_str()))
        {
            version_set.retain(|v| v != major);
        }
    }

    if version_set.is_empty() {
        return Ok("missing".to_string());
    }

    version_set.sort_by_key(|v| version_key(v));
    Ok(version_set.join(" | "))
}

fn version_key(v: &str) -> (u32, u32, u32) {
    let mut parts = v.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
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
    fn collapses_minor_versions_and_drops_the_bare_major() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": [
                "Programming Language :: Python :: 3",
                "Programming Language :: Python :: 3.9",
                "Programming Language :: Python :: 3.10"
            ]}}"#,
        );
        let value = resolve_python_versions(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "3.9 | 3.10");
    }

    #[test]
    fn keeps_bare_major_when_no_minor_versions_are_present() {
        let fetcher =
            FakeFetcher(r#"{"info": {"classifiers": ["Programming Language :: Python :: 3"]}}"#);
        let value = resolve_python_versions(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn falls_back_to_only_classifiers_when_no_plain_versions_exist() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": ["Programming Language :: Python :: 3 :: Only"]}}"#,
        );
        let value = resolve_python_versions(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "3");
    }

    #[test]
    fn reports_missing_when_no_python_classifiers_are_present() {
        let fetcher =
            FakeFetcher(r#"{"info": {"classifiers": ["License :: OSI Approved :: MIT License"]}}"#);
        let value = resolve_python_versions(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "missing");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_python_versions(&HashMap::new(), &Unused).is_err());
        assert!(resolve_python_versions(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_python_versions(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_classifiers_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"info": {}}"#);
        assert!(resolve_python_versions(&params("requests"), &fetcher).is_err());
    }
}
