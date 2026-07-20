use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_server(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("'server' parameter must not be empty".to_string());
    }
    let trimmed = value.trim_end_matches('/');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("'server' parameter must be an http(s) URL".to_string());
    }
    if trimmed
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
    {
        return Err("'server' parameter contains disallowed characters".to_string());
    }
    Ok(trimmed.to_string())
}

fn validate_build_id(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'build-id' parameter must not be empty".to_string());
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("'build-id' parameter contains disallowed characters".to_string());
    }
    Ok(value)
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// TeamCity's REST API requires authentication for most build configs, but
/// falls back to a `?guest=1` anonymous-access mode when no credentials are
/// supplied, which is the only mode this connector uses.
pub(crate) fn resolve_build(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let build_id = params
        .get("build-id")
        .ok_or("teamcity-build requires a data-build-id attribute")?;
    let build_id = validate_build_id(build_id)?;
    let server = match params.get("server") {
        Some(s) => validate_server(s)?,
        None => "https://teamcity.jetbrains.com".to_string(),
    };
    let verbosity = match params.get("verbosity").map(String::as_str) {
        Some(v @ ("s" | "e")) => v,
        Some(_) => return Err("teamcity-build data-verbosity must be 's' or 'e'".to_string()),
        None => "s",
    };

    let build_locator = format!("buildType:(id:{build_id})");
    let url = format!(
        "{server}/app/rest/builds/{}?guest=1",
        percent_encode(&build_locator)
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "teamcity response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let status = value
        .get("status")
        .ok_or("teamcity response missing status")?;
    let status = status
        .as_text()
        .ok_or_else(|| "status was not a plain value".to_string())?;

    if status == "SUCCESS" {
        return Ok("passing".to_string());
    }
    if verbosity == "e" {
        if let Some(status_text) = value.get("statusText").and_then(|v| v.as_text()) {
            return Ok(status_text.to_lowercase());
        }
    }
    Ok(status.to_lowercase())
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

    fn params(build_id: &str) -> HashMap<String, String> {
        HashMap::from([("build-id".to_string(), build_id.to_string())])
    }

    #[test]
    fn reports_passing_on_success_regardless_of_verbosity() {
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3Abt345)?guest=1",
            body: r#"{"status": "SUCCESS", "statusText": "Tests passed: 42"}"#,
        };
        let value = resolve_build(&params("bt345"), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn reports_lowercased_status_by_default_on_failure() {
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3Abt345)?guest=1",
            body: r#"{"status": "FAILURE", "statusText": "Tests failed: 3"}"#,
        };
        let value = resolve_build(&params("bt345"), &fetcher).unwrap();
        assert_eq!(value, "failure");
    }

    #[test]
    fn reports_lowercased_status_text_when_verbose() {
        let mut p = params("bt345");
        p.insert("verbosity".to_string(), "e".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3Abt345)?guest=1",
            body: r#"{"status": "FAILURE", "statusText": "Tests failed: 3"}"#,
        };
        let value = resolve_build(&p, &fetcher).unwrap();
        assert_eq!(value, "tests failed: 3");
    }

    #[test]
    fn uses_a_custom_server_when_provided() {
        let mut p = params("FileHelpersStable");
        p.insert("server".to_string(), "https://ci.example.com/".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://ci.example.com/app/rest/builds/buildType%3A(id%3AFileHelpersStable)?guest=1",
            body: r#"{"status": "SUCCESS", "statusText": "Success"}"#,
        };
        let value = resolve_build(&p, &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn requires_build_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid build-id")
            }
        }
        assert!(resolve_build(&HashMap::new(), &Unused).is_err());
        assert!(resolve_build(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_build_id() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid build-id")
            }
        }
        assert!(resolve_build(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn rejects_a_non_http_server() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid server")
            }
        }
        let mut p = params("bt345");
        p.insert("server".to_string(), "ftp://example.com".to_string());
        assert!(resolve_build(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_an_invalid_verbosity() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid verbosity")
            }
        }
        let mut p = params("bt345");
        p.insert("verbosity".to_string(), "x".to_string());
        assert!(resolve_build(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_status_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://teamcity.jetbrains.com/app/rest/builds/buildType%3A(id%3Abt345)?guest=1",
            body: r#"{}"#,
        };
        assert!(resolve_build(&params("bt345"), &fetcher).is_err());
    }
}
