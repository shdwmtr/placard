use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_package_name<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("'{name}' parameter must not be empty"));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | '/'))
    {
        return Err(format!("'{name}' parameter contains disallowed characters"));
    }
    Ok(value)
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub(crate) fn resolve_bundlephobia(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("bundlephobia requires a data-package attribute")?;
    let package = validate_package_name("package", package)?;

    let mut query = match params.get("scope") {
        Some(scope) if !scope.is_empty() => {
            let scope = validate_package_name("scope", scope)?;
            let scope = scope.strip_prefix('@').unwrap_or(scope);
            format!("@{scope}/{package}")
        }
        _ => package.to_string(),
    };
    if let Some(version) = params.get("version") {
        if !version.is_empty() {
            let version = validate_package_name("version", version)?;
            query.push('@');
            query.push_str(version);
        }
    }

    let format = params.get("format").map(String::as_str).unwrap_or("minzip");
    let field = match format {
        "min" => "size",
        "minzip" => "gzip",
        other => {
            return Err(format!(
                "'format' parameter must be one of min, minzip, got '{other}'"
            ));
        }
    };

    let url = format!(
        "https://bundlephobia.com/api/size?package={}",
        percent_encode(&query)
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "bundlephobia response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let size = value
        .get(field)
        .ok_or_else(|| format!("bundlephobia response missing {field}"))?;
    size.as_text()
        .ok_or_else(|| format!("{field} was not a plain value"))
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

    fn params(package: &str, extra: &[(&str, &str)]) -> HashMap<String, String> {
        let mut map = HashMap::from([("package".to_string(), package.to_string())]);
        for (k, v) in extra {
            map.insert((*k).to_string(), (*v).to_string());
        }
        map
    }

    #[test]
    fn defaults_to_gzip_size() {
        let fetcher = FakeFetcher {
            expected_url: "https://bundlephobia.com/api/size?package=react",
            body: r#"{"size": 100000, "gzip": 30000}"#,
        };
        let value = resolve_bundlephobia(&params("react", &[]), &fetcher).unwrap();
        assert_eq!(value, "30000");
    }

    #[test]
    fn min_format_uses_raw_size() {
        let fetcher = FakeFetcher {
            expected_url: "https://bundlephobia.com/api/size?package=react",
            body: r#"{"size": 100000, "gzip": 30000}"#,
        };
        let value = resolve_bundlephobia(&params("react", &[("format", "min")]), &fetcher).unwrap();
        assert_eq!(value, "100000");
    }

    #[test]
    fn includes_scope_and_version_in_the_query() {
        let fetcher = FakeFetcher {
            expected_url: "https://bundlephobia.com/api/size?package=%40cycle%2Fcore%407.0.0",
            body: r#"{"size": 500, "gzip": 200}"#,
        };
        let value = resolve_bundlephobia(
            &params("core", &[("scope", "@cycle"), ("version", "7.0.0")]),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "200");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_bundlephobia(&HashMap::new(), &Unused).is_err());
        assert!(resolve_bundlephobia(&params("", &[]), &Unused).is_err());
    }

    #[test]
    fn rejects_an_invalid_format() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_bundlephobia(&params("react", &[("format", "bogus")]), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://bundlephobia.com/api/size?package=react",
            body: r#"{"size": 100000}"#,
        };
        assert!(resolve_bundlephobia(&params("react", &[]), &fetcher).is_err());
    }
}
