use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn info_url(extension_type: &str, slug: &str) -> String {
    format!(
        "https://api.wordpress.org/{extension_type}s/info/1.2/?action={extension_type}_information&request[slug]={slug}&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1"
    )
}

pub(crate) fn resolve_platform(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let variant = params
        .get("variant")
        .map(String::as_str)
        .ok_or("wordpress-platform requires a data-variant attribute")?;
    let field = match variant {
        "requires" => "requires",
        "requires-php" => "requires_php",
        "tested" => "tested",
        other => return Err(format!("wordpress-platform unsupported variant '{other}'")),
    };

    let extension_type = if variant == "tested" {
        "plugin"
    } else {
        params.get("type").map(String::as_str).unwrap_or("plugin")
    };
    if extension_type != "plugin" && extension_type != "theme" {
        return Err("wordpress-platform 'type' parameter must be 'plugin' or 'theme'".to_string());
    }
    let slug = params
        .get("slug")
        .ok_or("wordpress-platform requires a data-slug attribute")?;
    let slug = validate_path_param("slug", slug)?;

    let url = info_url(extension_type, slug);
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "wordpress response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let field_value = value
        .get(field)
        .ok_or_else(|| format!("wordpress response missing {field}"))?;
    if let Value::Bool(false) = field_value {
        return Err(format!("{field} is not set for this {extension_type}"));
    }
    field_value
        .as_text()
        .ok_or_else(|| format!("{field} was not a plain value"))
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

    fn params(extension_type: &str, slug: &str, variant: &str) -> HashMap<String, String> {
        HashMap::from([
            ("type".to_string(), extension_type.to_string()),
            ("slug".to_string(), slug.to_string()),
            ("variant".to_string(), variant.to_string()),
        ])
    }

    #[test]
    fn extracts_requires_field() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]=bbpress&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "2.6.9", "requires": "5.3"}"#,
        );
        let value = resolve_platform(&params("plugin", "bbpress", "requires"), &fetcher).unwrap();
        assert_eq!(value, "5.3");
    }

    #[test]
    fn extracts_requires_php_field() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/themes/info/1.2/?action=theme_information&request[slug]=twentyseventeen&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "3.0", "requires_php": "7.0"}"#,
        );
        let value = resolve_platform(
            &params("theme", "twentyseventeen", "requires-php"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "7.0");
    }

    #[test]
    fn extracts_tested_field_always_via_plugin_endpoint() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]=bbpress&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "2.6.9", "tested": "6.4.3"}"#,
        );
        let value = resolve_platform(&params("theme", "bbpress", "tested"), &fetcher).unwrap();
        assert_eq!(value, "6.4.3");
    }

    #[test]
    fn errors_when_field_is_boolean_false() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]=bbpress&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "2.6.9", "requires": false}"#,
        );
        assert!(resolve_platform(&params("plugin", "bbpress", "requires"), &fetcher).is_err());
    }

    #[test]
    fn requires_variant_and_slug_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_platform(&HashMap::new(), &Unused).is_err());
        assert!(resolve_platform(&params("plugin", "", "requires"), &Unused).is_err());
        assert!(resolve_platform(&params("plugin", "bbpress", "bogus"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_platform(&params("plugin", "../etc", "requires"), &Unused).is_err());
    }
}
