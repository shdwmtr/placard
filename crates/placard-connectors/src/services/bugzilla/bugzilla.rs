use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn resolve_base_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("base-url") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'base-url' parameter must not be empty".to_string());
            }
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
        None => Ok("https://bugzilla.mozilla.org".to_string()),
    }
}

pub(crate) fn resolve_bugzilla(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let bug_number = params
        .get("bug-number")
        .ok_or("bugzilla requires a data-bug-number attribute")?;
    let bug_number = validate_path_param("bug-number", bug_number)?;
    let base_url = resolve_base_url(params)?;

    let url = format!("{base_url}/rest/bug/{bug_number}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "bugzilla response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let bugs = value.get("bugs").ok_or("bugzilla response missing bugs")?;
    let Value::Array(bugs) = bugs else {
        return Err("bugzilla response's bugs field was not an array".to_string());
    };
    let bug = bugs
        .first()
        .ok_or("bugzilla response's bugs array was empty")?;
    let status = bug
        .get("status")
        .and_then(json::Value::as_text)
        .ok_or("bugzilla bug missing status")?;
    let resolution = bug.get("resolution").and_then(json::Value::as_text);

    let display_status = if status == "RESOLVED" {
        resolution
            .ok_or("bugzilla bug missing resolution")?
            .to_lowercase()
    } else {
        status.to_lowercase()
    };
    let display_status = match display_status.as_str() {
        "worksforme" => "works for me".to_string(),
        "wontfix" => "won't fix".to_string(),
        other => other.to_string(),
    };
    Ok(display_status)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    fn params(bug_number: &str) -> HashMap<String, String> {
        HashMap::from([("bug-number".to_string(), bug_number.to_string())])
    }

    #[test]
    fn extracts_status_from_a_bugzilla_shaped_response() {
        let fetcher = FakeFetcher(
            "https://bugzilla.mozilla.org/rest/bug/12345",
            r#"{"bugs": [{"status": "NEW", "resolution": ""}]}"#,
        );
        let value = resolve_bugzilla(&params("12345"), &fetcher).unwrap();
        assert_eq!(value, "new");
    }

    #[test]
    fn uses_resolution_when_resolved() {
        let fetcher = FakeFetcher(
            "https://bugzilla.mozilla.org/rest/bug/12345",
            r#"{"bugs": [{"status": "RESOLVED", "resolution": "FIXED"}]}"#,
        );
        let value = resolve_bugzilla(&params("12345"), &fetcher).unwrap();
        assert_eq!(value, "fixed");
    }

    #[test]
    fn rewrites_worksforme_and_wontfix() {
        let fetcher = FakeFetcher(
            "https://bugzilla.mozilla.org/rest/bug/12345",
            r#"{"bugs": [{"status": "RESOLVED", "resolution": "WORKSFORME"}]}"#,
        );
        assert_eq!(
            resolve_bugzilla(&params("12345"), &fetcher).unwrap(),
            "works for me"
        );

        let fetcher = FakeFetcher(
            "https://bugzilla.mozilla.org/rest/bug/12345",
            r#"{"bugs": [{"status": "RESOLVED", "resolution": "WONTFIX"}]}"#,
        );
        assert_eq!(
            resolve_bugzilla(&params("12345"), &fetcher).unwrap(),
            "won't fix"
        );
    }

    #[test]
    fn uses_a_custom_base_url_when_provided() {
        let mut p = params("55");
        p.insert(
            "base-url".to_string(),
            "https://gcc.gnu.org/bugzilla".to_string(),
        );
        let fetcher = FakeFetcher(
            "https://gcc.gnu.org/bugzilla/rest/bug/55",
            r#"{"bugs": [{"status": "NEW", "resolution": ""}]}"#,
        );
        let value = resolve_bugzilla(&p, &fetcher).unwrap();
        assert_eq!(value, "new");
    }

    #[test]
    fn requires_bug_number_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_bugzilla(&HashMap::new(), &Unused).is_err());
        assert!(resolve_bugzilla(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_bugzilla(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_bugs_array_is_missing() {
        let fetcher = FakeFetcher(
            "https://bugzilla.mozilla.org/rest/bug/12345",
            r#"{"bugs": []}"#,
        );
        assert!(resolve_bugzilla(&params("12345"), &fetcher).is_err());
    }
}
