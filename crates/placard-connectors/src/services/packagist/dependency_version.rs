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

/// `dependency` can be a package name like `twig/twig` (with a slash) or a
/// plain platform identifier like `php`/`ext-xml`, and it's only ever used
/// as a lookup key into the response, never interpolated into the request
/// URL, so this just rules out empty/whitespace-only input.
fn validate_dependency(value: &str) -> Result<&str, String> {
    if value.trim().is_empty() {
        return Err("'dependency' parameter must not be empty".to_string());
    }
    Ok(value)
}

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

fn pick_latest_index(versions: &[Value]) -> Option<usize> {
    let has_version = |v: &&Value| v.get("version").and_then(|x| x.as_text()).is_some();
    if let Some(idx) = versions.iter().position(|v| {
        v.get("version")
            .and_then(|x| x.as_text())
            .is_some_and(|ver| is_stable_version(&ver))
    }) {
        return Some(idx);
    }
    versions.iter().position(|v| has_version(&v))
}

/// Packagist's `p2` API delta-encodes each version entry against the
/// previous one -- a field only appears when it changed, and a value of
/// the literal string `"__unset"` means "remove this field from the
/// running total". This replays those deltas into fully materialized
/// version objects, so `require` (often unchanged between releases) can be
/// read off any entry, not just the first.
fn expand_versions(raw: &[Value]) -> Vec<Value> {
    let mut current: Vec<(String, Value)> = Vec::new();
    let mut expanded = Vec::with_capacity(raw.len());
    for (i, entry) in raw.iter().enumerate() {
        let Value::Object(fields) = entry else {
            expanded.push(Value::Object(current.clone()));
            continue;
        };
        if i == 0 {
            current = fields.clone();
        } else {
            for (key, value) in fields {
                let is_unset = matches!(value, Value::String(s) if s == "__unset");
                current.retain(|(k, _)| k != key);
                if !is_unset {
                    current.push((key.clone(), value.clone()));
                }
            }
        }
        expanded.push(Value::Object(current.clone()));
    }
    expanded
}

pub(crate) fn resolve_dependency_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("packagist-dependency-version requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("packagist-dependency-version requires a data-repo attribute")?;
    let dependency = params
        .get("dependency")
        .ok_or("packagist-dependency-version requires a data-dependency attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let dependency = validate_dependency(dependency)?;
    let requested_version = params.get("version").filter(|v| !v.is_empty());
    let server = resolve_server_url(params)?;

    let package_name = format!("{}/{}", user.to_lowercase(), repo.to_lowercase());
    let url = format!("{server}/p2/{package_name}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "packagist response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let raw_versions = match find_package(&value, &package_name) {
        Some(Value::Array(items)) => items,
        _ => {
            return Err(format!(
                "packagist response missing packages.{package_name}"
            ));
        }
    };

    let idx = match requested_version {
        Some(version) => raw_versions
            .iter()
            .position(|v| {
                v.get("version").and_then(|x| x.as_text()).as_deref() == Some(version.as_str())
            })
            .ok_or("invalid version")?,
        None => pick_latest_index(raw_versions).ok_or("no released version found")?,
    };

    let expanded = expand_versions(raw_versions);
    let require = match expanded.get(idx).and_then(|v| v.get("require")) {
        Some(Value::Object(fields)) => fields,
        _ => return Err("version requirement not found".to_string()),
    };

    let dep_lower = dependency.to_lowercase();
    require
        .iter()
        .find(|(k, _)| k.to_lowercase() == dep_lower)
        .and_then(|(_, v)| v.as_text())
        .ok_or_else(|| "version requirement not found".to_string())
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

    fn params(user: &str, repo: &str, dependency: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("dependency".to_string(), dependency.to_string()),
        ])
    }

    #[test]
    fn extracts_the_dependency_constraint_from_the_latest_release() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.8.0", "require": {"php": "^7.2.5 || ^8.0", "psr/http-client": "^1.0"}}
            ]}}"#,
        );
        let value =
            resolve_dependency_version(&params("guzzlehttp", "Guzzle", "php"), &fetcher).unwrap();
        assert_eq!(value, "^7.2.5 || ^8.0");
    }

    #[test]
    fn matches_a_slash_containing_dependency_name_case_insensitively() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.8.0", "require": {"Psr/Http-Client": "^1.0"}}
            ]}}"#,
        );
        let value = resolve_dependency_version(
            &params("guzzlehttp", "guzzle", "psr/http-client"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "^1.0");
    }

    #[test]
    fn carries_require_forward_when_a_later_entry_omits_it() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.8.0", "require": {"php": "^7.2.5"}},
                {"version": "7.7.0"}
            ]}}"#,
        );
        let mut p = params("guzzlehttp", "guzzle", "php");
        p.insert("version".to_string(), "7.7.0".to_string());
        let value = resolve_dependency_version(&p, &fetcher).unwrap();
        assert_eq!(value, "^7.2.5");
    }

    #[test]
    fn errors_for_a_version_that_does_not_exist() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.8.0", "require": {"php": "^7.2.5"}}
            ]}}"#,
        );
        let mut p = params("guzzlehttp", "guzzle", "php");
        p.insert("version".to_string(), "999.0.0".to_string());
        assert!(resolve_dependency_version(&p, &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_dependency_is_not_required() {
        let fetcher = FakeFetcher(
            r#"{"packages": {"guzzlehttp/guzzle": [
                {"version": "7.8.0", "require": {"php": "^7.2.5"}}
            ]}}"#,
        );
        assert!(
            resolve_dependency_version(
                &params("guzzlehttp", "guzzle", "symfony/console"),
                &fetcher
            )
            .is_err()
        );
    }

    #[test]
    fn requires_user_repo_and_dependency_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_dependency_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_dependency_version(&params("guzzlehttp", "guzzle", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_dependency_version(&params("../etc", "guzzle", "php"), &Unused).is_err());
    }
}
