use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_framework_versions(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let framework = params
        .get("framework")
        .ok_or("pypi-framework-versions requires a data-framework attribute")?;
    let package = params
        .get("package")
        .ok_or("pypi-framework-versions requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let classifier = framework_classifier(framework).ok_or_else(|| {
        format!("'framework' parameter '{framework}' is not a supported framework")
    })?;

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

    let prefix = format!("Framework :: {classifier} :: ");
    let mut versions: Vec<String> = items
        .iter()
        .filter_map(|item| item.as_text())
        .filter_map(|s| s.strip_prefix(prefix.as_str()).map(|v| v.to_string()))
        .collect();

    if versions.is_empty() {
        return Err(format!("{framework} versions are missing for {package}"));
    }

    versions.sort_by_key(|v| parse_major_minor(v));
    Ok(versions.join(" | "))
}

fn framework_classifier(name: &str) -> Option<&'static str> {
    match name {
        "aws-cdk" => Some("AWS CDK"),
        "django" => Some("Django"),
        "django-cms" => Some("Django CMS"),
        "jupyterlab" => Some("Jupyter :: JupyterLab"),
        "odoo" => Some("Odoo"),
        "plone" => Some("Plone"),
        "wagtail" => Some("Wagtail"),
        "zope" => Some("Zope"),
        _ => None,
    }
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
            assert_eq!(url, "https://pypi.org/pypi/plone.volto/json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(framework: &str, package: &str) -> HashMap<String, String> {
        HashMap::from([
            ("framework".to_string(), framework.to_string()),
            ("package".to_string(), package.to_string()),
        ])
    }

    #[test]
    fn extracts_and_sorts_framework_versions() {
        let fetcher = FakeFetcher(
            r#"{"info": {"classifiers": [
                "Framework :: Plone :: 6.0",
                "Framework :: Plone :: 5.2"
            ]}}"#,
        );
        let value = resolve_framework_versions(&params("plone", "plone.volto"), &fetcher).unwrap();
        assert_eq!(value, "5.2 | 6.0");
    }

    #[test]
    fn handles_multi_segment_classifier_names() {
        struct JupyterFetcher;
        impl Fetcher for JupyterFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(url, "https://pypi.org/pypi/jupyterlab-git/json");
                Ok(
                    r#"{"info": {"classifiers": ["Framework :: Jupyter :: JupyterLab :: 4"]}}"#
                        .as_bytes()
                        .to_vec(),
                )
            }
        }
        let value =
            resolve_framework_versions(&params("jupyterlab", "jupyterlab-git"), &JupyterFetcher)
                .unwrap();
        assert_eq!(value, "4");
    }

    #[test]
    fn requires_framework_and_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_framework_versions(&HashMap::new(), &Unused).is_err());
        assert!(resolve_framework_versions(&params("plone", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_framework_names() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an unsupported framework")
            }
        }
        assert!(resolve_framework_versions(&params("flask", "plone.volto"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_framework_versions(&params("plone", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_matching_classifiers_are_present() {
        let fetcher =
            FakeFetcher(r#"{"info": {"classifiers": ["Programming Language :: Python :: 3"]}}"#);
        assert!(resolve_framework_versions(&params("plone", "plone.volto"), &fetcher).is_err());
    }
}
