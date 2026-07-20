use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_server(server: &str) -> Result<String, String> {
    if server.is_empty() {
        return Err("'base-url' parameter must not be empty".to_string());
    }
    let trimmed = server.trim_end_matches('/');
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

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(*n),
        _ => None,
    }
}

pub(crate) fn resolve_f_droid(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let app_id = params
        .get("app-id")
        .ok_or("f-droid requires a data-app-id attribute")?;
    let app_id = validate_path_param("app-id", app_id)?;
    let base_url = match params.get("base-url") {
        Some(base_url) => validate_server(base_url)?,
        None => "https://f-droid.org".to_string(),
    };

    let url = format!("{base_url}/api/v1/packages/{app_id}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "f-droid response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let suggested = !params.contains_key("include-prereleases");
    let svc = if suggested {
        value.get("suggestedVersionCode").and_then(as_number)
    } else {
        None
    };

    let packages = match value.get("packages") {
        Some(Value::Array(items)) => items.as_slice(),
        _ => &[],
    };

    let mut best: Option<(f64, String)> = None;
    for package in packages {
        let Some(version_code) = package.get("versionCode").and_then(as_number) else {
            continue;
        };
        if let Some(svc) = svc {
            if version_code > svc {
                continue;
            }
        }
        let Some(version_name) = package.get("versionName").and_then(|v| v.as_text()) else {
            continue;
        };
        if best
            .as_ref()
            .map(|(bc, _)| version_code > *bc)
            .unwrap_or(true)
        {
            best = Some((version_code, version_name));
        }
    }

    best.map(|(_, name)| name)
        .ok_or_else(|| "no packages found".to_string())
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

    fn params(app_id: &str) -> HashMap<String, String> {
        HashMap::from([("app-id".to_string(), app_id.to_string())])
    }

    #[test]
    fn extracts_the_suggested_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://f-droid.org/api/v1/packages/org.dystopia.email",
            body: r#"{
                "packageName": "org.dystopia.email",
                "suggestedVersionCode": 2,
                "packages": [
                    {"versionName": "1.0", "versionCode": 1},
                    {"versionName": "2.0", "versionCode": 2},
                    {"versionName": "3.0-beta", "versionCode": 3}
                ]
            }"#,
        };
        let value = resolve_f_droid(&params("org.dystopia.email"), &fetcher).unwrap();
        assert_eq!(value, "2.0");
    }

    #[test]
    fn includes_prereleases_when_requested() {
        let fetcher = FakeFetcher {
            expected_url: "https://f-droid.org/api/v1/packages/org.dystopia.email",
            body: r#"{
                "packageName": "org.dystopia.email",
                "suggestedVersionCode": 2,
                "packages": [
                    {"versionName": "1.0", "versionCode": 1},
                    {"versionName": "2.0", "versionCode": 2},
                    {"versionName": "3.0-beta", "versionCode": 3}
                ]
            }"#,
        };
        let mut p = params("org.dystopia.email");
        p.insert("include-prereleases".to_string(), String::new());
        let value = resolve_f_droid(&p, &fetcher).unwrap();
        assert_eq!(value, "3.0-beta");
    }

    #[test]
    fn uses_a_custom_base_url_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://apt.izzysoft.de/fdroid/api/v1/packages/org.dystopia.email",
            body: r#"{"packageName": "org.dystopia.email", "packages": [{"versionName": "1.0", "versionCode": 1}]}"#,
        };
        let mut p = params("org.dystopia.email");
        p.insert(
            "base-url".to_string(),
            "https://apt.izzysoft.de/fdroid/".to_string(),
        );
        let value = resolve_f_droid(&p, &fetcher).unwrap();
        assert_eq!(value, "1.0");
    }

    #[test]
    fn requires_an_app_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid app-id")
            }
        }
        assert!(resolve_f_droid(&HashMap::new(), &Unused).is_err());
        assert!(resolve_f_droid(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_app_id() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid app-id")
            }
        }
        assert!(resolve_f_droid(&params("a/b"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_packages_are_found() {
        let fetcher = FakeFetcher {
            expected_url: "https://f-droid.org/api/v1/packages/org.dystopia.email",
            body: r#"{"packageName": "org.dystopia.email", "packages": []}"#,
        };
        assert!(resolve_f_droid(&params("org.dystopia.email"), &fetcher).is_err());
    }
}
