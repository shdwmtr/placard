use crate::Fetcher;
use crate::json::{self, Value};
use crate::services::validate_path_param;
use std::collections::HashMap;

struct Release<'a> {
    version: &'a str,
    prerelease: bool,
}

fn extract_releases(root: &Value) -> Result<Vec<Release<'_>>, String> {
    let releases = match root.get("releases") {
        Some(Value::Object(fields)) => fields,
        _ => return Err("piwheels-version response missing releases".to_string()),
    };

    let mut out = Vec::new();
    for (version, entry) in releases {
        let Value::Object(entry_fields) = entry else {
            continue;
        };
        let yanked = matches!(
            entry_fields
                .iter()
                .find(|(k, _)| k == "yanked")
                .map(|(_, v)| v),
            Some(Value::Bool(true))
        );
        if yanked {
            continue;
        }
        let has_files = matches!(
            entry_fields.iter().find(|(k, _)| k == "files").map(|(_, v)| v),
            Some(Value::Object(files)) if !files.is_empty()
        );
        if !has_files {
            continue;
        }
        let prerelease = matches!(
            entry_fields
                .iter()
                .find(|(k, _)| k == "prerelease")
                .map(|(_, v)| v),
            Some(Value::Bool(true))
        );
        out.push(Release {
            version,
            prerelease,
        });
    }
    Ok(out)
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let wheel = params
        .get("wheel")
        .ok_or("piwheels-version requires a data-wheel attribute")?;
    let wheel = validate_path_param("wheel", wheel)?;
    let include_prereleases = params.contains_key("include_prereleases");

    let url = format!("https://www.piwheels.org/project/{wheel}/json/");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "piwheels response was not valid UTF-8".to_string())?;
    let root = json::parse(&text)?;
    let releases = extract_releases(&root)?;
    if releases.is_empty() {
        return Err("no versions found".to_string());
    }

    if include_prereleases {
        return Ok(releases[0].version.to_string());
    }

    releases
        .iter()
        .find(|r| !r.prerelease)
        .or_else(|| releases.first())
        .map(|r| r.version.to_string())
        .ok_or_else(|| "no versions found".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://www.piwheels.org/project/flask/json/");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(wheel: &str) -> HashMap<String, String> {
        HashMap::from([("wheel".to_string(), wheel.to_string())])
    }

    fn body() -> &'static str {
        r#"{
            "releases": {
                "2.1.0": {"prerelease": false, "yanked": true, "files": {"a": {}}},
                "2.0.0rc1": {"prerelease": true, "yanked": false, "files": {"a": {}}},
                "1.9.0": {"prerelease": false, "yanked": false, "files": {}},
                "1.8.0": {"prerelease": false, "yanked": false, "files": {"a": {}}},
                "1.7.0": {"prerelease": false, "yanked": false, "files": {"a": {}}}
            }
        }"#
    }

    #[test]
    fn picks_the_first_non_yanked_stable_release_with_files() {
        let fetcher = FakeFetcher(body());
        let value = resolve_version(&params("flask"), &fetcher).unwrap();
        assert_eq!(value, "1.8.0");
    }

    #[test]
    fn includes_prereleases_when_requested() {
        let fetcher = FakeFetcher(body());
        let mut p = params("flask");
        p.insert("include_prereleases".to_string(), String::new());
        let value = resolve_version(&p, &fetcher).unwrap();
        assert_eq!(value, "2.0.0rc1");
    }

    #[test]
    fn requires_wheel_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_usable_releases_exist() {
        let fetcher = FakeFetcher(
            r#"{"releases": {"1.0.0": {"prerelease": false, "yanked": true, "files": {"a": {}}}}}"#,
        );
        assert!(resolve_version(&params("flask"), &fetcher).is_err());
    }
}
