use crate::Fetcher;
use crate::json;
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

pub(crate) fn resolve_unpacked_size(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npm-unpacked-size requires a data-package attribute")?;
    let segment = package_url_segment(package)?;
    let base = registry_base(params);
    let version = params
        .get("version")
        .map(String::as_str)
        .unwrap_or("latest");
    validate_name_segment("version", version)?;

    let url = format!("{base}/{segment}/{version}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "npm response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;
    let size = doc
        .get("dist.unpackedSize")
        .ok_or("npm response missing dist.unpackedSize")?;
    size.as_text()
        .ok_or_else(|| "unpackedSize was not a plain value".to_string())
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
    fn extracts_unpacked_size_for_the_latest_version_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/npm/latest",
            body: r#"{"dist": {"unpackedSize": 123456}}"#,
        };
        let value = resolve_unpacked_size(&params("npm"), &fetcher).unwrap();
        assert_eq!(value, "123456");
    }

    #[test]
    fn uses_a_specific_version_when_given() {
        let mut p = params("npm");
        p.insert("version".to_string(), "4.18.2".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/npm/4.18.2",
            body: r#"{"dist": {"unpackedSize": 654321}}"#,
        };
        let value = resolve_unpacked_size(&p, &fetcher).unwrap();
        assert_eq!(value, "654321");
    }

    #[test]
    fn encodes_scoped_packages_with_percent_2f() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/@cedx%2Fgulp-david/latest",
            body: r#"{"dist": {"unpackedSize": 999}}"#,
        };
        let value = resolve_unpacked_size(&params("@cedx/gulp-david"), &fetcher).unwrap();
        assert_eq!(value, "999");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_unpacked_size(&HashMap::new(), &Unused).is_err());
        assert!(resolve_unpacked_size(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_unpacked_size(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_unpacked_size_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://registry.npmjs.org/npm/latest",
            body: r#"{"dist": {}}"#,
        };
        assert!(resolve_unpacked_size(&params("npm"), &fetcher).is_err());
    }
}
