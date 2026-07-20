use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_base_url(base_url: &str) -> Result<String, String> {
    if base_url.is_empty() {
        return Err("'base-url' parameter must not be empty".to_string());
    }
    let trimmed = base_url.trim_end_matches('/');
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err("'base-url' parameter must be an http(s) URL".to_string());
    }
    if trimmed
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
    {
        return Err("'base-url' parameter contains disallowed characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// Gerrit prefixes its JSON responses with a magic XSSI-prevention line
/// that must be stripped before parsing.
/// See <https://gerrit-review.googlesource.com/Documentation/rest-api.html#output>.
fn strip_xssi_prefix(text: &str) -> &str {
    match text.find('\n') {
        Some(idx) => &text[idx + 1..],
        None => text,
    }
}

pub(crate) fn resolve_gerrit(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let change_id = params
        .get("change-id")
        .ok_or("gerrit requires a data-change-id attribute")?;
    let change_id = validate_path_param("change-id", change_id)?;
    let base_url = params
        .get("base-url")
        .ok_or("gerrit requires a data-base-url attribute")?;
    let base_url = validate_base_url(base_url)?;

    let url = format!("{base_url}/changes/{change_id}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "gerrit response was not valid UTF-8".to_string())?;
    let value = json::parse(strip_xssi_prefix(&text))?;
    value
        .get("status")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "gerrit response missing status".to_string())
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

    fn params(change_id: &str, base_url: &str) -> HashMap<String, String> {
        HashMap::from([
            ("change-id".to_string(), change_id.to_string()),
            ("base-url".to_string(), base_url.to_string()),
        ])
    }

    #[test]
    fn strips_the_xssi_prefix_and_extracts_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://android-review.googlesource.com/changes/1011478",
            body: ")]}'\n{\"status\": \"MERGED\"}",
        };
        let value = resolve_gerrit(
            &params("1011478", "https://android-review.googlesource.com"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "MERGED");
    }

    #[test]
    fn requires_change_id_and_base_url() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_gerrit(&HashMap::new(), &Unused).is_err());
        assert!(resolve_gerrit(&params("1011478", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_disallowed_param_values() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(
            resolve_gerrit(
                &params("../etc", "https://android-review.googlesource.com"),
                &Unused
            )
            .is_err()
        );
        assert!(resolve_gerrit(&params("1011478", "not-a-url"), &Unused).is_err());
    }

    #[test]
    fn errors_when_status_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://android-review.googlesource.com/changes/1011478",
            body: ")]}'\n{\"id\": \"x\"}",
        };
        assert!(
            resolve_gerrit(
                &params("1011478", "https://android-review.googlesource.com"),
                &fetcher
            )
            .is_err()
        );
    }
}
