use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_scope(value: &str) -> Result<&str, String> {
    let rest = value
        .strip_prefix('@')
        .ok_or_else(|| "'scope' parameter must start with '@'".to_string())?;
    validate_path_param("scope", rest)?;
    Ok(value)
}

fn project_url(params: &HashMap<String, String>) -> Result<String, String> {
    let platform = params
        .get("platform")
        .ok_or("librariesio-sourcerank requires a data-platform attribute")?;
    let package_name = params
        .get("package-name")
        .ok_or("librariesio-sourcerank requires a data-package-name attribute")?;
    let platform = validate_path_param("platform", platform)?;
    let package_name = validate_path_param("package-name", package_name)?;

    let scope_segment = match params.get("scope") {
        Some(scope) if !scope.is_empty() => format!("{}/", validate_scope(scope)?),
        _ => String::new(),
    };

    Ok(format!(
        "https://libraries.io/api/{platform}/{scope_segment}{package_name}"
    ))
}

pub(crate) fn resolve_sourcerank(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = project_url(params)?;
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "libraries.io response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("rank")
        .ok_or("libraries.io response missing rank")?
        .as_text()
        .ok_or_else(|| "rank was not a plain value".to_string())
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

    fn params(platform: &str, package_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("platform".to_string(), platform.to_string()),
            ("package-name".to_string(), package_name.to_string()),
        ])
    }

    #[test]
    fn extracts_the_rank() {
        let fetcher = FakeFetcher {
            expected_url: "https://libraries.io/api/npm/lodash",
            body: r#"{"platform": "NPM", "dependent_repos_count": 44640, "dependents_count": 133226, "rank": 33}"#,
        };
        let value = resolve_sourcerank(&params("npm", "lodash"), &fetcher).unwrap();
        assert_eq!(value, "33");
    }

    #[test]
    fn builds_the_url_with_a_scope() {
        let mut p = params("npm", "core");
        p.insert("scope".to_string(), "@babel".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://libraries.io/api/npm/@babel/core",
            body: r#"{"platform": "NPM", "dependent_repos_count": 10, "dependents_count": 5, "rank": 20}"#,
        };
        let value = resolve_sourcerank(&p, &fetcher).unwrap();
        assert_eq!(value, "20");
    }

    #[test]
    fn requires_platform_and_package_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_sourcerank(&HashMap::new(), &Unused).is_err());
        assert!(resolve_sourcerank(&params("npm", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_sourcerank(&params("../etc", "lodash"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://libraries.io/api/npm/lodash",
            body: r#"{"platform": "NPM"}"#,
        };
        assert!(resolve_sourcerank(&params("npm", "lodash"), &fetcher).is_err());
    }
}
