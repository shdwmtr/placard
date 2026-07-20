use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn info_url(extension_type: &str, slug: &str) -> String {
    format!(
        "https://api.wordpress.org/{extension_type}s/info/1.2/?action={extension_type}_information&request[slug]={slug}&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1"
    )
}

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let extension_type = params.get("type").map(String::as_str).unwrap_or("plugin");
    if extension_type != "plugin" && extension_type != "theme" {
        return Err("wordpress-downloads 'type' parameter must be 'plugin' or 'theme'".to_string());
    }
    let slug = params
        .get("slug")
        .ok_or("wordpress-downloads requires a data-slug attribute")?;
    let slug = validate_path_param("slug", slug)?;
    let interval = params.get("interval").map(String::as_str).unwrap_or("dt");

    if interval == "dt" {
        let url = info_url(extension_type, slug);
        let bytes = fetcher.fetch(&url)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| "wordpress response was not valid UTF-8".to_string())?;
        let value = json::parse(&text)?;
        let downloaded = value
            .get("downloaded")
            .ok_or("wordpress response missing downloaded")?;
        downloaded
            .as_text()
            .ok_or_else(|| "downloaded was not a plain value".to_string())
    } else {
        let limit = match interval {
            "dd" => 1,
            "dw" => 7,
            "dm" => 30,
            "dy" => 365,
            other => {
                return Err(format!(
                    "wordpress-downloads unsupported interval '{other}'"
                ));
            }
        };
        let stats_type = if extension_type == "plugin" {
            "plugin"
        } else {
            "themes"
        };
        let url = format!(
            "https://api.wordpress.org/stats/{stats_type}/1.0/downloads.php?slug={slug}&limit={limit}"
        );
        let bytes = fetcher.fetch(&url)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| "wordpress response was not valid UTF-8".to_string())?;
        let value = json::parse(&text)?;
        let Value::Object(fields) = value else {
            return Err("wordpress downloads response was not an object".to_string());
        };
        if fields.is_empty() {
            return Err("wordpress downloads response was empty".to_string());
        }
        let mut total = 0i64;
        for (_, v) in &fields {
            match v {
                Value::Number(n) => total += *n as i64,
                _ => {
                    return Err(
                        "wordpress downloads response contained a non-numeric value".to_string()
                    );
                }
            }
        }
        Ok(total.to_string())
    }
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
    fn extracts_total_downloads_by_default() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]=bbpress&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "2.6.9", "downloaded": 5000000}"#,
        );
        let value = resolve_downloads(&params("plugin", "bbpress"), &fetcher).unwrap();
        assert_eq!(value, "5000000");
    }

    #[test]
    fn sums_interval_stats_when_an_interval_is_given() {
        let mut p = params("theme", "twentyseventeen");
        p.insert("interval".to_string(), "dw".to_string());
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/stats/themes/1.0/downloads.php?slug=twentyseventeen&limit=7",
            r#"{"2024-01-01": 10, "2024-01-02": 20}"#,
        );
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "30");
    }

    #[test]
    fn requires_slug_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("plugin", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("plugin", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_downloaded_field_is_missing() {
        let fetcher = FakeFetcher(
            "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]=bbpress&request[fields][active_installs]=1&request[fields][sections]=0&request[fields][homepage]=0&request[fields][tags]=0&request[fields][screenshot_url]=0&request[fields][downloaded]=1&request[fields][last_updated]=1&request[fields][requires_php]=1",
            r#"{"version": "2.6.9"}"#,
        );
        assert!(resolve_downloads(&params("plugin", "bbpress"), &fetcher).is_err());
    }
}
