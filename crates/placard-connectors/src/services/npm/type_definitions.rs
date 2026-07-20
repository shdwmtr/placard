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

fn files_contains(files: Option<&Value>, name: &str) -> bool {
    match files {
        Some(Value::Array(items)) => items
            .iter()
            .any(|v| matches!(v, Value::String(s) if s == name)),
        _ => false,
    }
}

pub(crate) fn resolve_type_definitions(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npm-type-definitions requires a data-package attribute")?;
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

    let dev_deps = version_data.get("devDependencies");
    let has_typescript_dep = dev_deps.and_then(|d| d.get("typescript")).is_some();
    let has_flow_dep = dev_deps.and_then(|d| obj_get(d, "flow-bin")).is_some();
    let has_types = version_data.get("types").is_some();
    let has_typings = version_data.get("typings").is_some();
    let files = version_data.get("files");
    let has_dts_file = files_contains(files, "index.d.ts");
    let has_flow_file = files_contains(files, "index.js.flow");

    let mut supported_languages = Vec::new();
    if has_types || has_typings || has_typescript_dep || has_dts_file {
        supported_languages.push("TypeScript");
    }
    if has_flow_dep || has_flow_file {
        supported_languages.push("Flow");
    }

    if supported_languages.is_empty() {
        Ok("none".to_string())
    } else {
        supported_languages.sort_unstable();
        Ok(supported_languages.join(" | "))
    }
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
    fn detects_typescript_via_the_types_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/chalk",
            body: r#"{"dist-tags": {"latest": "5.3.0"}, "versions": {"5.3.0": {"types": "index.d.ts", "devDependencies": {}, "files": []}}}"#,
        };
        let value = resolve_type_definitions(&params("chalk"), &fetcher).unwrap();
        assert_eq!(value, "TypeScript");
    }

    #[test]
    fn detects_typescript_via_devdependency_and_flow_via_files() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/some-lib",
            body: r#"{"dist-tags": {"latest": "1.0.0"}, "versions": {"1.0.0": {"devDependencies": {"typescript": "^5.0.0"}, "files": ["index.js.flow"]}}}"#,
        };
        let value = resolve_type_definitions(&params("some-lib"), &fetcher).unwrap();
        assert_eq!(value, "Flow | TypeScript");
    }

    #[test]
    fn returns_none_when_no_types_are_detected() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/plain-js",
            body: r#"{"dist-tags": {"latest": "1.0.0"}, "versions": {"1.0.0": {"devDependencies": {}, "files": ["index.js"]}}}"#,
        };
        let value = resolve_type_definitions(&params("plain-js"), &fetcher).unwrap();
        assert_eq!(value, "none");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_type_definitions(&HashMap::new(), &Unused).is_err());
        assert!(resolve_type_definitions(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_type_definitions(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_response_is_malformed() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/chalk",
            body: r#"{"dist-tags": {}}"#,
        };
        assert!(resolve_type_definitions(&params("chalk"), &fetcher).is_err());
    }
}
