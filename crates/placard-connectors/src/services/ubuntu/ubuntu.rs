use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_ubuntu(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("ubuntu requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let mut url = format!(
        "https://api.launchpad.net/1.0/ubuntu/+archive/primary?ws.op=getPublishedSources&exact_match=true&order_by_date=true&status=Published&source_name={package_name}"
    );
    if let Some(series) = params.get("series") {
        let series = validate_path_param("series", series)?;
        url.push_str("&distro_series=https%3A%2F%2Fapi.launchpad.net%2F1.0%2Fubuntu%2F");
        url.push_str(series);
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "ubuntu response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let entries = value
        .get("entries")
        .ok_or("ubuntu response missing entries")?;
    let Value::Array(entries) = entries else {
        return Err("ubuntu entries was not an array".to_string());
    };
    let first = entries.first().ok_or("package not found")?;
    first
        .get("source_package_version")
        .ok_or("ubuntu entry missing source_package_version")?
        .as_text()
        .ok_or_else(|| "source_package_version was not a plain value".to_string())
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

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_the_source_package_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.launchpad.net/1.0/ubuntu/+archive/primary?ws.op=getPublishedSources&exact_match=true&order_by_date=true&status=Published&source_name=ubuntu-wallpapers",
            body: r#"{"entries": [{"source_package_version": "20.04-0ubuntu1"}]}"#,
        };
        let value = resolve_ubuntu(&params("ubuntu-wallpapers"), &fetcher).unwrap();
        assert_eq!(value, "20.04-0ubuntu1");
    }

    #[test]
    fn includes_the_distro_series_param_when_series_given() {
        let mut p = params("ubuntu-wallpapers");
        p.insert("series".to_string(), "bionic".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://api.launchpad.net/1.0/ubuntu/+archive/primary?ws.op=getPublishedSources&exact_match=true&order_by_date=true&status=Published&source_name=ubuntu-wallpapers&distro_series=https%3A%2F%2Fapi.launchpad.net%2F1.0%2Fubuntu%2Fbionic",
            body: r#"{"entries": [{"source_package_version": "18.04-0ubuntu1"}]}"#,
        };
        let value = resolve_ubuntu(&p, &fetcher).unwrap();
        assert_eq!(value, "18.04-0ubuntu1");
    }

    #[test]
    fn requires_a_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_ubuntu(&HashMap::new(), &Unused).is_err());
        assert!(resolve_ubuntu(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_ubuntu(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_entries_are_returned() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.launchpad.net/1.0/ubuntu/+archive/primary?ws.op=getPublishedSources&exact_match=true&order_by_date=true&status=Published&source_name=nonexistent",
            body: r#"{"entries": []}"#,
        };
        assert!(resolve_ubuntu(&params("nonexistent"), &fetcher).is_err());
    }
}
