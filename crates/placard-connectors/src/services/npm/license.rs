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

fn license_text(value: &Value) -> Option<String> {
    match value {
        Value::String(_) => value.as_text(),
        Value::Object(_) => obj_get(value, "type").and_then(Value::as_text),
        Value::Array(items) => items.first().and_then(license_text),
        _ => None,
    }
}

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npm-license requires a data-package attribute")?;
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
    let license = version_data
        .get("license")
        .ok_or("npm response missing license")?;
    license_text(license).ok_or_else(|| "license was not a recognizable value".to_string())
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

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_a_plain_string_license() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/express",
            body: r#"{"dist-tags": {"latest": "4.18.2"}, "versions": {"4.18.2": {"license": "MIT"}}}"#,
        };
        let value = resolve_license(&params("express"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn extracts_the_type_field_from_a_deprecated_license_object() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/some-old-package",
            body: r#"{"dist-tags": {"latest": "0.1.0"}, "versions": {"0.1.0": {"license": {"type": "ISC"}}}}"#,
        };
        let value = resolve_license(&params("some-old-package"), &fetcher).unwrap();
        assert_eq!(value, "ISC");
    }

    #[test]
    fn takes_the_first_entry_from_a_license_array() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/some-package",
            body: r#"{"dist-tags": {"latest": "0.1.0"}, "versions": {"0.1.0": {"license": ["MIT", "Apache-2.0"]}}}"#,
        };
        let value = resolve_license(&params("some-package"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_license(&HashMap::new(), &Unused).is_err());
        assert!(resolve_license(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_license(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_license_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/express",
            body: r#"{"dist-tags": {"latest": "4.18.2"}, "versions": {"4.18.2": {}}}"#,
        };
        assert!(resolve_license(&params("express"), &fetcher).is_err());
    }
}
