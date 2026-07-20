use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

/// The `remote` param is a multi-segment path (e.g. `github.com/fsfe/reuse-tool`),
/// so it's validated one `/`-separated segment at a time.
fn validate_segmented_param<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("'{name}' parameter must not be empty"));
    }
    for segment in value.split('/') {
        validate_path_param(name, segment)?;
    }
    Ok(value)
}

pub(crate) fn resolve_compliance(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let remote = params
        .get("remote")
        .ok_or("reuse-compliance requires a data-remote attribute")?;
    let remote = validate_segmented_param("remote", remote)?;

    let url = format!("https://api.reuse.software/status/{remote}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "reuse response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let status = value.get("status").ok_or("reuse response missing status")?;
    status
        .as_text()
        .ok_or_else(|| "status was not a plain value".to_string())
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

    fn params(remote: &str) -> HashMap<String, String> {
        HashMap::from([("remote".to_string(), remote.to_string())])
    }

    #[test]
    fn extracts_the_compliance_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.reuse.software/status/github.com/fsfe/reuse-tool",
            body: r#"{"status": "compliant"}"#,
        };
        let value = resolve_compliance(&params("github.com/fsfe/reuse-tool"), &fetcher).unwrap();
        assert_eq!(value, "compliant");
    }

    #[test]
    fn requires_remote_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_compliance(&HashMap::new(), &Unused).is_err());
        assert!(resolve_compliance(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_compliance(&params("github.com/fsfe/reuse tool"), &Unused).is_err());
        assert!(resolve_compliance(&params("github.com/fsfe/reuse?tool"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_status_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.reuse.software/status/github.com/fsfe/reuse-tool",
            body: r#"{"other": 1}"#,
        };
        assert!(resolve_compliance(&params("github.com/fsfe/reuse-tool"), &fetcher).is_err());
    }
}
