use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn info_url(extension_type: &str, slug: &str) -> String {
    format!(
        "https://api.wordpress.org/{extension_type}s/info/1.2/?action={extension_type}_information&request[slug]={slug}&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1"
    )
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let extension_type = params.get("type").map(String::as_str).unwrap_or("plugin");
    if extension_type != "plugin" && extension_type != "theme" {
        return Err("wordpress-version 'type' parameter must be 'plugin' or 'theme'".to_string());
    }
    let slug = params
        .get("slug")
        .ok_or("wordpress-version requires a data-slug attribute")?;
    let slug = validate_path_param("slug", slug)?;

    let url = info_url(extension_type, slug);
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "wordpress response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("wordpress response missing version")?;
    version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())
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

    fn params(extension_type: &str, slug: &str) -> HashMap<String, String> {
        HashMap::from([
            ("type".to_string(), extension_type.to_string()),
            ("slug".to_string(), slug.to_string()),
        ])
    }

    #[test]
    fn extracts_version_field() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]=bbpress&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "2.6.9", "rating": 90}"#,
        );
        let value = resolve_version(&params("plugin", "bbpress"), &fetcher).unwrap();
        assert_eq!(value, "2.6.9");
    }

    #[test]
    fn uses_theme_endpoint_for_theme_type() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/themes/info/1.2/?action=theme_information&request[slug]=twentyseventeen&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "3.1"}"#,
        );
        let value = resolve_version(&params("theme", "twentyseventeen"), &fetcher).unwrap();
        assert_eq!(value, "3.1");
    }

    #[test]
    fn requires_slug_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("plugin", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("plugin", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_version_field_is_missing() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]=bbpress&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"rating": 90}"#,
        );
        assert!(resolve_version(&params("plugin", "bbpress"), &fetcher).is_err());
    }
}
