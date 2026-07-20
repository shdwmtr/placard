use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pypi-license requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://pypi.org/pypi/{package}/json");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "pypi response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let licenses = get_licenses(&value)?;

    Ok(if licenses.is_empty() {
        "missing".to_string()
    } else {
        licenses.join(", ")
    })
}

fn get_licenses(value: &Value) -> Result<Vec<String>, String> {
    let info = value.get("info").ok_or("pypi response missing info")?;

    if let Some(expr) = info
        .get("license_expression")
        .and_then(Value::as_text)
        .filter(|s| !s.is_empty())
    {
        return Ok(vec![expr]);
    }

    if let Some(license) = info
        .get("license")
        .and_then(Value::as_text)
        .filter(|s| !s.is_empty())
    {
        if license.chars().count() < 40 {
            return Ok(vec![license]);
        }
    }

    let classifiers = info
        .get("classifiers")
        .ok_or("pypi response missing info.classifiers")?;
    let Value::Array(items) = classifiers else {
        return Err("pypi response 'info.classifiers' was not an array".to_string());
    };

    let mut licenses: Vec<String> = Vec::new();
    for item in items {
        let Some(text) = item.as_text() else { continue };
        let Some(rest) = text.strip_prefix("License :: ") else {
            continue;
        };
        let mapped = spdx_alias(rest)
            .map(str::to_string)
            .unwrap_or_else(|| rest.to_string());
        let segment = mapped.split(" :: ").last().unwrap_or(&mapped).to_string();
        let stripped = replace_first(&segment, " License", "");
        let normalized = if let Some(inner) = extract_parenthesized(&stripped) {
            inner.to_uppercase()
        } else if !stripped.is_empty()
            && stripped
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            stripped.to_uppercase()
        } else {
            stripped
        };
        licenses.push(normalized);
    }

    if licenses.len() > 1 {
        licenses.retain(|l| l != "DFSG approved");
    }

    Ok(licenses)
}

fn spdx_alias(s: &str) -> Option<&'static str> {
    match s {
        "OSI Approved :: Apache Software License" => Some("Apache-2.0"),
        "CC0 1.0 Universal (CC0 1.0) Public Domain Dedication" => Some("CC0-1.0"),
        "OSI Approved :: GNU Affero General Public License v3" => Some("AGPL-3.0"),
        "OSI Approved :: Zero-Clause BSD (0BSD)" => Some("0BSD"),
        _ => None,
    }
}

fn extract_parenthesized(s: &str) -> Option<&str> {
    let start = s.find('(')?;
    let end = s[start..].find(')')? + start;
    Some(&s[start + 1..end])
}

fn replace_first(s: &str, from: &str, to: &str) -> String {
    match s.find(from) {
        Some(pos) => {
            let mut out = String::with_capacity(s.len());
            out.push_str(&s[..pos]);
            out.push_str(to);
            out.push_str(&s[pos + from.len()..]);
            out
        }
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://pypi.org/pypi/requests/json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn prefers_license_expression_when_present() {
        let fetcher = FakeFetcher(
            r#"{"info": {"license_expression": "MIT OR Apache-2.0", "license": "", "classifiers": []}}"#,
        );
        let value = resolve_license(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "MIT OR Apache-2.0");
    }

    #[test]
    fn falls_back_to_a_short_license_field() {
        let fetcher = FakeFetcher(r#"{"info": {"license": "Apache-2.0", "classifiers": []}}"#);
        let value = resolve_license(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "Apache-2.0");
    }

    #[test]
    fn falls_back_to_trove_classifiers_when_license_is_a_long_text_blob() {
        let fetcher = FakeFetcher(
            r#"{"info": {"license": "This is the full text of a license and is definitely over forty characters long.", "classifiers": ["License :: OSI Approved :: MIT License"]}}"#,
        );
        let value = resolve_license(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn maps_known_classifiers_via_spdx_aliases() {
        let fetcher = FakeFetcher(
            r#"{"info": {"license": null, "classifiers": ["License :: OSI Approved :: Apache Software License"]}}"#,
        );
        let value = resolve_license(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "Apache-2.0");
    }

    #[test]
    fn reports_missing_when_no_license_information_is_found() {
        let fetcher = FakeFetcher(r#"{"info": {"license": "", "classifiers": []}}"#);
        let value = resolve_license(&params("requests"), &fetcher).unwrap();
        assert_eq!(value, "missing");
    }

    #[test]
    fn requires_package_param() {
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
        assert!(resolve_license(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_info_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_license(&params("requests"), &fetcher).is_err());
    }
}
