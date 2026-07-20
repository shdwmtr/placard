use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn registry_base(params: &HashMap<String, String>) -> String {
    match params.get("registry_uri") {
        Some(v) if !v.is_empty() => v.trim_end_matches('/').to_string(),
        _ => "https://registry.npmjs.org".to_string(),
    }
}

fn encode_slug(package: &str) -> Result<String, String> {
    if let Some(rest) = package.strip_prefix('@') {
        let mut parts = rest.splitn(2, '/');
        let scope = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or("'package' scope must not be empty")?;
        let name = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or("'package' must include a name after the scope, e.g. @scope/name")?;
        let scope = validate_path_param("package", scope)?;
        let name = validate_path_param("package", name)?;
        Ok(format!("@{scope}%2F{name}"))
    } else {
        let name = validate_path_param("package", package)?;
        Ok(name.to_string())
    }
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn fetch_package_doc(
    registry: &str,
    slug: &str,
    tag: Option<&str>,
    fetcher: &dyn Fetcher,
) -> Result<Value, String> {
    match tag {
        None => {
            let url = format!("{registry}/{slug}/latest");
            let bytes = fetcher.fetch(&url)?;
            let text = String::from_utf8(bytes)
                .map_err(|_| "npm response was not valid UTF-8".to_string())?;
            json::parse(&text)
        }
        Some(tag) => {
            let url = format!("{registry}/{slug}");
            let bytes = fetcher.fetch(&url)?;
            let text = String::from_utf8(bytes)
                .map_err(|_| "npm response was not valid UTF-8".to_string())?;
            let full = json::parse(&text)?;
            let dist_tags = field(&full, "dist-tags").ok_or("npm response missing dist-tags")?;
            let version = field(dist_tags, tag)
                .ok_or_else(|| format!("npm response missing the '{tag}' dist-tag"))?
                .as_text()
                .ok_or_else(|| "dist-tag value was not a plain value".to_string())?;
            let versions = field(&full, "versions").ok_or("npm response missing versions")?;
            field(versions, &version)
                .cloned()
                .ok_or_else(|| "npm response missing the resolved version".to_string())
        }
    }
}

/// Reports the `engines.node` range from the package's `package.json`,
/// which is what the badge's message is built from upstream. The color
/// comparison against Node's current release (`versionColorForRangeCurrent`)
/// is badge-drawing logic with no connector equivalent, so it's skipped.
pub(crate) fn resolve_current(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("node-current requires a data-package attribute")?;
    let slug = encode_slug(package)?;
    let registry = registry_base(params);
    let tag = params.get("tag").map(String::as_str);

    let doc = fetch_package_doc(&registry, &slug, tag, fetcher)?;
    let engines = field(&doc, "engines").ok_or("npm response missing engines")?;
    let node_range = field(engines, "node").ok_or("npm response missing engines.node")?;
    node_range
        .as_text()
        .ok_or_else(|| "engines.node was not a plain value".to_string())
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
    fn extracts_engines_node_from_the_latest_version_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/passport/latest",
            body: r#"{"engines": {"node": ">= 0.4.0"}}"#,
        };
        let value = resolve_current(&params("passport"), &fetcher).unwrap();
        assert_eq!(value, ">= 0.4.0");
    }

    #[test]
    fn resolves_a_named_tag_through_dist_tags_and_versions() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/passport",
            body: r#"{"dist-tags": {"next": "1.0.0"}, "versions": {"1.0.0": {"engines": {"node": ">=18"}}}}"#,
        };
        let mut p = params("passport");
        p.insert("tag".to_string(), "next".to_string());
        let value = resolve_current(&p, &fetcher).unwrap();
        assert_eq!(value, ">=18");
    }

    #[test]
    fn encodes_scoped_package_names() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/@cedx%2Fgulp-david/latest",
            body: r#"{"engines": {"node": ">=14"}}"#,
        };
        let value = resolve_current(&params("@cedx/gulp-david"), &fetcher).unwrap();
        assert_eq!(value, ">=14");
    }

    #[test]
    fn honors_a_custom_registry_uri() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.example.com/passport/latest",
            body: r#"{"engines": {"node": ">=10"}}"#,
        };
        let mut p = params("passport");
        p.insert(
            "registry_uri".to_string(),
            "https://registry.example.com/".to_string(),
        );
        let value = resolve_current(&p, &fetcher).unwrap();
        assert_eq!(value, ">=10");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package")
            }
        }
        assert!(resolve_current(&HashMap::new(), &Unused).is_err());
        assert!(resolve_current(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid package")
            }
        }
        assert!(resolve_current(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_engines_node_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/passport/latest",
            body: r#"{"engines": {}}"#,
        };
        assert!(resolve_current(&params("passport"), &fetcher).is_err());
    }
}
