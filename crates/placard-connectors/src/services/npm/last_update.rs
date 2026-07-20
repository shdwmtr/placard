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

pub(crate) fn resolve_last_update(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npm-last-update requires a data-package attribute")?;
    let segment = package_url_segment(package)?;
    let base = registry_base(params);
    let tag = params.get("tag").map(String::as_str).unwrap_or("latest");
    if tag.is_empty() {
        return Err("'tag' parameter must not be empty".to_string());
    }

    let url = format!("{base}/{segment}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "npm response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;

    let dist_tags = doc
        .get("dist-tags")
        .ok_or("npm response missing dist-tags")?;
    let version = obj_get(dist_tags, tag).ok_or_else(|| format!("tag '{tag}' not found"))?;
    let version = version
        .as_text()
        .ok_or_else(|| "dist-tags value was not a plain value".to_string())?;

    let time = doc.get("time").ok_or("npm response missing time")?;
    let updated =
        obj_get(time, &version).ok_or_else(|| format!("npm response missing time.{version}"))?;
    updated
        .as_text()
        .ok_or_else(|| "time value was not a plain value".to_string())
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
    fn extracts_the_modified_timestamp_of_the_latest_dist_tag() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/verdaccio",
            body: r#"{"dist-tags": {"latest": "5.0.0"}, "time": {"5.0.0": "2024-05-01T12:00:00.000Z", "modified": "2024-05-01T12:00:00.000Z"}}"#,
        };
        let value = resolve_last_update(&params("verdaccio"), &fetcher).unwrap();
        assert_eq!(value, "2024-05-01T12:00:00.000Z");
    }

    #[test]
    fn uses_a_custom_tag_when_given() {
        let mut p = params("verdaccio");
        p.insert("tag".to_string(), "next-8".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/verdaccio",
            body: r#"{"dist-tags": {"latest": "5.0.0", "next-8": "6.0.0-next.8"}, "time": {"5.0.0": "2024-05-01T12:00:00.000Z", "6.0.0-next.8": "2024-06-01T09:00:00.000Z"}}"#,
        };
        let value = resolve_last_update(&p, &fetcher).unwrap();
        assert_eq!(value, "2024-06-01T09:00:00.000Z");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_last_update(&HashMap::new(), &Unused).is_err());
        assert!(resolve_last_update(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_last_update(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_tag_is_not_found() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/verdaccio",
            body: r#"{"dist-tags": {"latest": "5.0.0"}, "time": {"5.0.0": "2024-05-01T12:00:00.000Z"}}"#,
        };
        let mut p = params("verdaccio");
        p.insert("tag".to_string(), "next-8".to_string());
        assert!(resolve_last_update(&p, &fetcher).is_err());
    }
}
