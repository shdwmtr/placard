use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

fn base_url(params: &HashMap<String, String>) -> String {
    match params.get("registry_uri") {
        Some(v) if !v.is_empty() => v.trim_end_matches('/').to_string(),
        _ => DEFAULT_REGISTRY.to_string(),
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

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npm-version requires a data-package attribute")?;
    let slug = encode_slug(package)?;
    let registry = base_url(params);

    let url = format!("{registry}/-/package/{slug}/dist-tags");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "npm response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let tag = params.get("tag").map(String::as_str).unwrap_or("latest");
    let version = value
        .get(tag)
        .ok_or_else(|| format!("npm response missing the '{tag}' dist-tag"))?;
    version
        .as_text()
        .ok_or_else(|| "dist-tag value was not a plain value".to_string())
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
    fn extracts_the_latest_dist_tag_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/-/package/npm/dist-tags",
            body: r#"{"latest": "10.8.2", "next-8": "8.0.0-beta.1"}"#,
        };
        let value = resolve_version(&params("npm"), &fetcher).unwrap();
        assert_eq!(value, "10.8.2");
    }

    #[test]
    fn extracts_a_named_dist_tag_when_requested() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/-/package/npm/dist-tags",
            body: r#"{"latest": "10.8.2", "next-8": "8.0.0-beta.1"}"#,
        };
        let mut p = params("npm");
        p.insert("tag".to_string(), "next-8".to_string());
        let value = resolve_version(&p, &fetcher).unwrap();
        assert_eq!(value, "8.0.0-beta.1");
    }

    #[test]
    fn encodes_scoped_package_names() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/-/package/@cedx%2Fgulp-david/dist-tags",
            body: r#"{"latest": "1.2.3"}"#,
        };
        let value = resolve_version(&params("@cedx/gulp-david"), &fetcher).unwrap();
        assert_eq!(value, "1.2.3");
    }

    #[test]
    fn honors_a_custom_registry_uri() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.example.com/-/package/npm/dist-tags",
            body: r#"{"latest": "1.0.0"}"#,
        };
        let mut p = params("npm");
        p.insert(
            "registry_uri".to_string(),
            "https://registry.example.com/".to_string(),
        );
        let value = resolve_version(&p, &fetcher).unwrap();
        assert_eq!(value, "1.0.0");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid package")
            }
        }
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
        assert!(resolve_version(&params("@scope/"), &Unused).is_err());
        assert!(resolve_version(&params("@/name"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_requested_tag_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/-/package/npm/dist-tags",
            body: r#"{"latest": "10.8.2"}"#,
        };
        let mut p = params("npm");
        p.insert("tag".to_string(), "nonexistent".to_string());
        assert!(resolve_version(&p, &fetcher).is_err());
    }
}
