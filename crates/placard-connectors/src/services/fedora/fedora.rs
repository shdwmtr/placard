use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_fedora(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("fedora requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;
    let branch = params
        .get("branch")
        .map(String::as_str)
        .unwrap_or("rawhide");
    let branch = validate_path_param("branch", branch)?;

    let url = format!("https://apps.fedoraproject.org/mdapi/{branch}/pkg/{package_name}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "fedora response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("version")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "fedora response missing version".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: &'static str,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_the_version_using_the_default_branch() {
        let fetcher = FakeFetcher {
            expected_url: "https://apps.fedoraproject.org/mdapi/rawhide/pkg/rpm",
            body: r#"{"version": "4.18.0"}"#,
        };
        let value = resolve_fedora(&params("rpm"), &fetcher).unwrap();
        assert_eq!(value, "4.18.0");
    }

    #[test]
    fn uses_a_custom_branch_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://apps.fedoraproject.org/mdapi/f40/pkg/rpm",
            body: r#"{"version": "4.19.0"}"#,
        };
        let mut p = params("rpm");
        p.insert("branch".to_string(), "f40".to_string());
        let value = resolve_fedora(&p, &fetcher).unwrap();
        assert_eq!(value, "4.19.0");
    }

    #[test]
    fn requires_a_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package-name")
            }
        }
        assert!(resolve_fedora(&HashMap::new(), &Unused).is_err());
        assert!(resolve_fedora(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_fedora(&params("../etc"), &Unused).is_err());
        let mut p = params("rpm");
        p.insert("branch".to_string(), "a/b".to_string());
        assert!(resolve_fedora(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_version_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://apps.fedoraproject.org/mdapi/rawhide/pkg/rpm",
            body: r#"{"name": "rpm"}"#,
        };
        assert!(resolve_fedora(&params("rpm"), &fetcher).is_err());
    }
}
