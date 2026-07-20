use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_django_versions(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-django-versions requires a data-package attribute")?;
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

    let prefix = "Framework :: Django :: ";
    let mut versions: Vec<String> = items
        .iter()
        .filter_map(|item| item.as_text())
        .filter_map(|s| s.strip_prefix(prefix).map(|v| v.to_string()))
        .collect();

    if versions.is_empty() {
        return Err(format!("Django versions are missing for {package}"));
    }

    versions.sort_by_key(|v| parse_major_minor(v));
    Ok(versions.join(" | "))
}

fn parse_major_minor(v: &str) -> (u32, u32) {
    let mut parts = v.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://pypi.org/pypi/django-taggit/json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_and_sorts_django_versions() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": [
                "Framework :: Django :: 3.2",
                "Framework :: Django :: 2.2",
                "Framework :: Django :: 4.0"
            ]}}"#,
        );
        let value = resolve_django_versions(&params("django-taggit"), &fetcher).unwrap();
        assert_eq!(value, "2.2 | 3.2 | 4.0");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_django_versions(&HashMap::new(), &Unused).is_err());
        assert!(resolve_django_versions(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_django_versions(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_django_classifiers_are_present() {
        let fetcher =
            FakeFetcher(r#"{"info": {"classifiers": ["Programming Language :: Python :: 3"]}}"#);
        assert!(resolve_django_versions(&params("django-taggit"), &fetcher).is_err());
    }
}
