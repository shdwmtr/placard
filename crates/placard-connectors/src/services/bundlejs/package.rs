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

pub(crate) fn resolve_package(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("bundlejs-package requires a data-package attribute")?;
    let package = validate_package_name("package", package)?;

    let query = match params.get("scope") {
        Some(scope) if !scope.is_empty() => {
            let scope = validate_package_name("scope", scope)?;
            let scope = scope.strip_prefix('@').unwrap_or(scope);
            format!("@{scope}/{package}")
        }
        _ => package.to_string(),
    };

    let url = format!("https://deno.bundlejs.com?q={}", percent_encode(&query));
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "bundlejs response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let size = value
        .get("size.rawCompressedSize")
        .ok_or("bundlejs response missing size.rawCompressedSize")?;
    size.as_text()
        .ok_or_else(|| "size.rawCompressedSize was not a plain value".to_string())
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

    fn params(package: &str, scope: Option<&str>) -> HashMap<String, String> {
        let mut map = HashMap::from([("package".to_string(), package.to_string())]);
        if let Some(scope) = scope {
            map.insert("scope".to_string(), scope.to_string());
        }
        map
    }

    #[test]
    fn extracts_compressed_size_from_a_bundlejs_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://deno.bundlejs.com?q=value-enhancer%403.1.2",
            body: r#"{"size": {"rawCompressedSize": 1234, "rawUncompressedSize": 4321}}"#,
        };
        let value = resolve_package(&params("value-enhancer@3.1.2", None), &fetcher).unwrap();
        assert_eq!(value, "1234");
    }

    #[test]
    fn includes_a_scope_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://deno.bundlejs.com?q=%40ngneat%2Ffalso%406.4.0",
            body: r#"{"size": {"rawCompressedSize": 99, "rawUncompressedSize": 200}}"#,
        };
        let value = resolve_package(&params("falso@6.4.0", Some("@ngneat")), &fetcher).unwrap();
        assert_eq!(value, "99");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_package(&HashMap::new(), &Unused).is_err());
        assert!(resolve_package(&params("", None), &Unused).is_err());
    }

    #[test]
    fn rejects_disallowed_characters_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_package(&params("valid pkg", None), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://deno.bundlejs.com?q=react",
            body: r#"{"size": {"rawUncompressedSize": 200}}"#,
        };
        assert!(resolve_package(&params("react", None), &fetcher).is_err());
    }
}
