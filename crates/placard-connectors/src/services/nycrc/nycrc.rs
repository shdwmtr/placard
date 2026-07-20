use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_segmented_param<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("'{name}' parameter must not be empty"));
    }
    for segment in value.split('/') {
        validate_path_param(name, segment)?;
    }
    Ok(value)
}

fn obj_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(*n),
        _ => None,
    }
}

const VALID_THRESHOLDS: [&str; 3] = ["branches", "lines", "functions"];

/// Mirrors shields' `extractThreshold`: an explicit `preferredThreshold`
/// wins if present, otherwise branches are favored over lines (the
/// stricter of the two), falling back to lines if branches are absent.
fn extract_threshold(config: &Value, preferred: Option<&str>) -> Result<f64, String> {
    if let Some(pref) = preferred {
        if !VALID_THRESHOLDS.contains(&pref) {
            return Err(
                "'preferred-threshold' must be \"branches\", \"lines\", or \"functions\""
                    .to_string(),
            );
        }
        return obj_get(config, pref)
            .and_then(as_number)
            .ok_or_else(|| format!("\"{pref}\" threshold missing"));
    }
    if let Some(v) = obj_get(config, "branches").and_then(as_number) {
        return Ok(v);
    }
    if let Some(v) = obj_get(config, "lines").and_then(as_number) {
        return Ok(v);
    }
    Err("\"branches\" or \"lines\" threshold missing".to_string())
}

pub(crate) fn resolve_nycrc(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("nycrc requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("nycrc requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;
    let config = match params.get("config") {
        Some(config) if !config.is_empty() => validate_segmented_param("config", config)?,
        _ => ".nycrc",
    };
    let preferred = params.get("preferred-threshold").map(String::as_str);

    let url = format!("https://raw.githubusercontent.com/{user}/{repo}/HEAD/{config}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "nycrc response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;

    let config_obj = if config.contains("package.json") {
        obj_get(&doc, "c8")
            .or_else(|| obj_get(&doc, "nyc"))
            .ok_or("no nyc or c8 stanza found")?
    } else {
        &doc
    };

    let coverage = extract_threshold(config_obj, preferred)?;
    Ok(format!("{coverage:.0}%"))
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

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn defaults_to_the_dot_nycrc_file_and_favors_branches() {
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/yargs/yargs/HEAD/.nycrc",
            body: r#"{"branches": 90, "lines": 95}"#,
        };
        let value = resolve_nycrc(&params("yargs", "yargs"), &fetcher).unwrap();
        assert_eq!(value, "90%");
    }

    #[test]
    fn falls_back_to_lines_when_branches_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/yargs/yargs/HEAD/.nycrc",
            body: r#"{"lines": 95}"#,
        };
        let value = resolve_nycrc(&params("yargs", "yargs"), &fetcher).unwrap();
        assert_eq!(value, "95%");
    }

    #[test]
    fn honors_an_explicit_preferred_threshold() {
        let mut p = params("yargs", "yargs");
        p.insert("preferred-threshold".to_string(), "lines".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/yargs/yargs/HEAD/.nycrc",
            body: r#"{"branches": 90, "lines": 95}"#,
        };
        let value = resolve_nycrc(&p, &fetcher).unwrap();
        assert_eq!(value, "95%");
    }

    #[test]
    fn reads_thresholds_from_a_custom_config_file() {
        let mut p = params("yargs", "yargs");
        p.insert("config".to_string(), "coverage.json".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/yargs/yargs/HEAD/coverage.json",
            body: r#"{"branches": 80}"#,
        };
        let value = resolve_nycrc(&p, &fetcher).unwrap();
        assert_eq!(value, "80%");
    }

    #[test]
    fn reads_the_nyc_or_c8_stanza_from_package_json() {
        let mut p = params("yargs", "yargs");
        p.insert("config".to_string(), "package.json".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/yargs/yargs/HEAD/package.json",
            body: r#"{"name": "yargs", "c8": {"branches": 70}}"#,
        };
        let value = resolve_nycrc(&p, &fetcher).unwrap();
        assert_eq!(value, "70%");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_nycrc(&HashMap::new(), &Unused).is_err());
        assert!(resolve_nycrc(&params("yargs", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_nycrc(&params("../etc", "yargs"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_threshold_is_present() {
        let fetcher = FakeFetcher {
            expected_url: "https://raw.githubusercontent.com/yargs/yargs/HEAD/.nycrc",
            body: r#"{"functions": 50}"#,
        };
        assert!(resolve_nycrc(&params("yargs", "yargs"), &fetcher).is_err());
    }
}
