use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

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

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let extension_id = params
        .get("extension-id")
        .ok_or("gnome-extensions-downloads requires a data-extension-id attribute")?;
    let extension_id = validate_extension_id(extension_id)?;

    let url = format!("https://extensions.gnome.org/api/v1/extensions/{extension_id}/");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "gnome-extensions response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let downloads = value
        .get("downloads")
        .ok_or("gnome-extensions response missing downloads")?;
    downloads
        .as_text()
        .ok_or_else(|| "downloads was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://extensions.gnome.org/api/v1/extensions/just-perfection-desktop@just-perfection/"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(extension_id: &str) -> HashMap<String, String> {
        HashMap::from([("extension-id".to_string(), extension_id.to_string())])
    }

    #[test]
    fn extracts_downloads_from_a_gnome_extensions_shaped_response() {
        let fetcher = FakeFetcher(r#"{"downloads": 42000}"#);
        let value = resolve_downloads(&params("just-perfection-desktop@just-perfection"), &fetcher)
            .unwrap();
        assert_eq!(value, "42000");
    }

    #[test]
    fn requires_extension_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc/passwd"), &Unused).is_err());
        assert!(resolve_downloads(&params("a/b"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"other": 1}"#);
        assert!(
            resolve_downloads(&params("just-perfection-desktop@just-perfection"), &fetcher)
                .is_err()
        );
    }
}
