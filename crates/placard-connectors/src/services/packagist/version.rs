use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn resolve_server_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("server") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'server' parameter must not be empty".to_string());
            }
            if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
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
        None => Ok("https://repo.packagist.org".to_string()),
    }
}

/// A version is treated as a pre-release if it carries a `dev`, `alpha`,
/// `beta`, or `rc` modifier -- a plain substring check, not a full semver
/// comparator, since versions only need to be classified individually here
/// (never compared against one another).
fn is_stable_version(version: &str) -> bool {
    let lower = version.to_lowercase();
    !(lower.starts_with("dev-")
        || lower.contains("-dev")
        || lower.contains(".dev")
        || lower.contains("alpha")
        || lower.contains("beta")
        || lower.contains("-rc")
        || lower.contains(".rc")
        || lower.starts_with("rc"))
}

/// Picks the index of the version packagist considers "latest": the packagist
/// `p2` API already returns versions newest-first, so this is the first
/// stable entry (or, if none is stable, simply the first entry) -- unless
/// `include_prereleases` is set, in which case it's just the first entry.
/// Looks up `packages.<package_name>` directly by key rather than through
/// `Value::get`'s dotted-path walk, since a package name may itself contain
/// a `.` (composer allows it as a separator), which would otherwise be
/// misread as a path boundary.
fn find_package<'a>(value: &'a Value, package_name: &str) -> Option<&'a Value> {
    match value.get("packages") {
        Some(Value::Object(fields)) => fields
            .iter()
            .find(|(k, _)| k == package_name)
            .map(|(_, v)| v),
        _ => None,
    }
}

fn pick_latest_index(versions: &[Value], include_prereleases: bool) -> Option<usize> {
    let has_version = |v: &&Value| v.get("version").and_then(|x| x.as_text()).is_some();
    if include_prereleases {
        return versions.iter().position(|v| has_version(&v));
    }
    if let Some(idx) = versions.iter().position(|v| {
        v.get("version")
            .and_then(|x| x.as_text())
            .is_some_and(|ver| is_stable_version(&ver))
    }) {
        return Some(idx);
    }
    versions.iter().position(|v| has_version(&v))
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("packagist-version requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("packagist-version requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let include_prereleases = params.contains_key("include_prereleases");
    let server = resolve_server_url(params)?;

    let package_name = format!("{}/{}", user.to_lowercase(), repo.to_lowercase());
    let url = format!("{server}/p2/{package_name}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "packagist response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let versions = match find_package(&value, &package_name) {
        Some(Value::Array(items)) => items,
        _ => {
            return Err(format!(
                "packagist response missing packages.{package_name}"
            ));
        }
    };

    let idx =
        pick_latest_index(versions, include_prereleases).ok_or("no released version found")?;
    versions[idx]
        .get("version")
        .and_then(|v| v.as_text())
        .ok_or_else(|| "version entry missing version field".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://repo.packagist.org/p2/guzzlehttp/guzzle.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_first_stable_version() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.9.0-beta1"},
                {"version": "7.8.0"},
                {"version": "7.7.0"}
            ]}}"#,
        );
        let value = resolve_version(&params("guzzlehttp", "Guzzle"), &fetcher).unwrap();
        assert_eq!(value, "7.8.0");
    }

    #[test]
    fn includes_prereleases_when_requested() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.9.0-beta1"},
                {"version": "7.8.0"}
            ]}}"#,
        );
        let mut p = params("guzzlehttp", "guzzle");
        p.insert("include_prereleases".to_string(), String::new());
        let value = resolve_version(&p, &fetcher).unwrap();
        assert_eq!(value, "7.9.0-beta1");
    }

    #[test]
    fn falls_back_to_first_entry_when_nothing_is_stable() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.9.0-beta1"},
                {"version": "7.8.0-alpha1"}
            ]}}"#,
        );
        let value = resolve_version(&params("guzzlehttp", "guzzle"), &fetcher).unwrap();
        assert_eq!(value, "7.9.0-beta1");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("guzzlehttp", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc", "guzzle"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_package_is_missing_from_the_response() {
        let fetcher = FakeFetcher(r#"{"packages": {}}"#);
        assert!(resolve_version(&params("guzzlehttp", "guzzle"), &fetcher).is_err());
    }
}
