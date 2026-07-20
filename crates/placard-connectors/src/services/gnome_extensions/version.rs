use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

const ACTIVE_STATUS: f64 = 3.0;

fn validate_extension_id(id: &str) -> Result<&str, String> {
    if id.is_empty() {
        return Err("'extension-id' parameter must not be empty".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@')
    {
        return Err("'extension-id' parameter contains disallowed characters".to_string());
    }
    Ok(id)
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let extension_id = params
        .get("extension-id")
        .ok_or("gnome-extensions-version requires a data-extension-id attribute")?;
    let extension_id = validate_extension_id(extension_id)?;

    let url = format!(
        "https://extensions.gnome.org/api/v1/extensions/{extension_id}/versions/?page_size=100"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "gnome-extensions response was not valid UTF-8".to_string())?;
    let root = json::parse(&text)?;
    let results = root
        .get("results")
        .ok_or("gnome-extensions response missing results")?;
    let Value::Array(items) = results else {
        return Err("gnome-extensions response's results field was not an array".to_string());
    };

    let mut latest: Option<(f64, &Value)> = None;
    for item in items {
        let Some(Value::Number(status)) = item.get("status") else {
            continue;
        };
        if *status != ACTIVE_STATUS {
            continue;
        }
        let Some(Value::Number(version_num)) = item.get("version") else {
            continue;
        };
        if latest.map(|(best, _)| *version_num > best).unwrap_or(true) {
            latest = Some((*version_num, item));
        }
    }

    let (version_num, latest) = latest.ok_or("no active version found")?;
    if let Some(name) = latest.get("version_name").and_then(Value::as_text) {
        Ok(name)
    } else {
        json::Value::Number(version_num)
            .as_text()
            .ok_or_else(|| "version was not a plain value".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://extensions.gnome.org/api/v1/extensions/just-perfection-desktop@just-perfection/versions/?page_size=100"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(extension_id: &str) -> HashMap<String, String> {
        HashMap::from([("extension-id".to_string(), extension_id.to_string())])
    }

    #[test]
    fn picks_the_highest_active_version_name() {
        let fetcher = FakeFetcher(
            r#"{"results": [
                {"version": 10, "version_name": "1.0", "status": 3},
                {"version": 20, "version_name": "2.0", "status": 3},
                {"version": 30, "version_name": "3.0", "status": 1}
            ]}"#,
        );
        let value =
            resolve_version(&params("just-perfection-desktop@just-perfection"), &fetcher).unwrap();
        assert_eq!(value, "2.0");
    }

    #[test]
    fn falls_back_to_the_numeric_version_when_no_name() {
        let fetcher =
            FakeFetcher(r#"{"results": [{"version": 15, "version_name": null, "status": 3}]}"#);
        let value =
            resolve_version(&params("just-perfection-desktop@just-perfection"), &fetcher).unwrap();
        assert_eq!(value, "15");
    }

    #[test]
    fn requires_extension_id_param() {
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
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_active_version_exists() {
        let fetcher =
            FakeFetcher(r#"{"results": [{"version": 10, "version_name": "1.0", "status": 1}]}"#);
        assert!(
            resolve_version(&params("just-perfection-desktop@just-perfection"), &fetcher).is_err()
        );
    }
}
