use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn registry_base(params: &HashMap<String, String>) -> String {
    match params.get("registry_uri") {
        Some(v) if !v.is_empty() => v.trim_end_matches('/').to_string(),
        _ => "https://registry.npmjs.org".to_string(),
    }
}

fn validate_name_segment(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(format!("'{name}' parameter contains disallowed characters"));
    }
    Ok(())
}

fn package_url_segment(package: &str) -> Result<String, String> {
    if let Some(rest) = package.strip_prefix('@') {
        let mut parts = rest.splitn(2, '/');
        let scope = parts.next().unwrap_or("");
        let name = parts
            .next()
            .ok_or_else(|| "'package' parameter must be in the form @scope/name".to_string())?;
        validate_name_segment("package", scope)?;
        validate_name_segment("package", name)?;
        Ok(format!("@{scope}%2F{name}"))
    } else {
        validate_name_segment("package", package)?;
        Ok(package.to_string())
    }
}

fn obj_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn latest_version(doc: &Value) -> Result<String, String> {
    let dist_tags = doc
        .get("dist-tags")
        .ok_or("npm response missing dist-tags")?;
    let latest = obj_get(dist_tags, "latest").ok_or("npm response missing dist-tags.latest")?;
    latest
        .as_text()
        .ok_or_else(|| "dist-tags.latest was not a plain value".to_string())
}

pub(crate) fn resolve_dependency_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npm-dependency-version requires a data-package attribute")?;
    let dependency = params
        .get("dependency")
        .ok_or("npm-dependency-version requires a data-dependency attribute")?;
    if dependency.is_empty() {
        return Err("'dependency' parameter must not be empty".to_string());
    }
    let kind = params.get("kind").map(String::as_str).unwrap_or("prod");
    let field_name = match kind {
        "dev" => "devDependencies",
        "peer" => "peerDependencies",
        "prod" => "dependencies",
        other => return Err(format!("unknown dependency kind '{other}'")),
    };

    let segment = package_url_segment(package)?;
    let base = registry_base(params);
    let url = format!("{base}/{segment}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "npm response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;
    let version = latest_version(&doc)?;
    let versions = doc.get("versions").ok_or("npm response missing versions")?;
    let version_data = obj_get(versions, &version)
        .ok_or_else(|| format!("npm response missing versions.{version}"))?;
    let deps = version_data
        .get(field_name)
        .ok_or_else(|| format!("npm response missing {field_name}"))?;
    let range = obj_get(deps, dependency)
        .ok_or_else(|| format!("'{dependency}' not found in {field_name}"))?;
    range
        .as_text()
        .ok_or_else(|| "dependency range was not a plain value".to_string())
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

    fn params(package: &str, dependency: &str) -> HashMap<String, String> {
        HashMap::from([
            ("package".to_string(), package.to_string()),
            ("dependency".to_string(), dependency.to_string()),
        ])
    }

    #[test]
    fn extracts_a_prod_dependency_range_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/react-boxplot",
            body: r#"{"dist-tags": {"latest": "1.0.0"}, "versions": {"1.0.0": {"dependencies": {"simple-statistics": "^7.7.0"}}}}"#,
        };
        let value =
            resolve_dependency_version(&params("react-boxplot", "simple-statistics"), &fetcher)
                .unwrap();
        assert_eq!(value, "^7.7.0");
    }

    #[test]
    fn extracts_a_dev_dependency_range_when_kind_is_dev() {
        let mut p = params("react-boxplot", "prop-types");
        p.insert("kind".to_string(), "dev".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/react-boxplot",
            body: r#"{"dist-tags": {"latest": "1.0.0"}, "versions": {"1.0.0": {"devDependencies": {"prop-types": "~15.8.1"}}}}"#,
        };
        let value = resolve_dependency_version(&p, &fetcher).unwrap();
        assert_eq!(value, "~15.8.1");
    }

    #[test]
    fn handles_dependency_names_containing_dots() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/some-app",
            body: r#"{"dist-tags": {"latest": "2.5.0"}, "versions": {"2.5.0": {"dependencies": {"lodash.get": "^4.4.2"}}}}"#,
        };
        let value =
            resolve_dependency_version(&params("some-app", "lodash.get"), &fetcher).unwrap();
        assert_eq!(value, "^4.4.2");
    }

    #[test]
    fn requires_package_and_dependency_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_dependency_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_dependency_version(&params("react-boxplot", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_package_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(
            resolve_dependency_version(&params("../etc/passwd", "simple-statistics"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_dependency_is_not_present() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/react-boxplot",
            body: r#"{"dist-tags": {"latest": "1.0.0"}, "versions": {"1.0.0": {"dependencies": {}}}}"#,
        };
        assert!(
            resolve_dependency_version(&params("react-boxplot", "simple-statistics"), &fetcher)
                .is_err()
        );
    }
}
