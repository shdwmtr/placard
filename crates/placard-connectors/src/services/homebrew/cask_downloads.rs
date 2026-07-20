use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn obj_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn interval_field(interval: &str) -> Result<&'static str, String> {
    match interval {
        "dm" => Ok("30d"),
        "dq" => Ok("90d"),
        "dy" => Ok("365d"),
        other => Err(format!(
            "unknown interval '{other}', expected one of dm, dq, dy"
        )),
    }
}

pub(crate) fn resolve_cask_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let cask = params
        .get("cask")
        .ok_or("homebrew-cask-downloads requires a data-cask attribute")?;
    let cask = validate_path_param("cask", cask)?;
    let interval = params.get("interval").map(String::as_str).unwrap_or("dm");
    let field = interval_field(interval)?;

    let url = format!("https://formulae.brew.sh/api/cask/{cask}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "homebrew response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;
    let install = doc
        .get("analytics.install")
        .ok_or("homebrew response missing analytics.install")?;
    let period = obj_get(install, field)
        .ok_or_else(|| format!("homebrew response missing analytics.install.{field}"))?;
    let downloads = obj_get(period, cask)
        .ok_or_else(|| format!("'{cask}' not found in analytics.install.{field}"))?;
    downloads
        .as_text()
        .ok_or_else(|| "download count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://formulae.brew.sh/api/cask/freetube.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(cask: &str, interval: Option<&str>) -> HashMap<String, String> {
        let mut m = HashMap::from([("cask".to_string(), cask.to_string())]);
        if let Some(i) = interval {
            m.insert("interval".to_string(), i.to_string());
        }
        m
    }

    #[test]
    fn extracts_monthly_downloads_by_default() {
        let fetcher = FakeFetcher(
            r#"{"analytics": {"install": {"30d": {"freetube": 1000}, "90d": {"freetube": 3000}, "365d": {"freetube": 12000}}}}"#,
        );
        let value = resolve_cask_downloads(&params("freetube", None), &fetcher).unwrap();
        assert_eq!(value, "1000");
    }

    #[test]
    fn extracts_yearly_downloads_when_interval_is_dy() {
        let fetcher = FakeFetcher(
            r#"{"analytics": {"install": {"30d": {"freetube": 1000}, "90d": {"freetube": 3000}, "365d": {"freetube": 12000}}}}"#,
        );
        let value = resolve_cask_downloads(&params("freetube", Some("dy")), &fetcher).unwrap();
        assert_eq!(value, "12000");
    }

    #[test]
    fn requires_cask_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_cask_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_cask_downloads(&params("", None), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_cask_downloads(&params("../etc/passwd", None), &Unused).is_err());
    }

    #[test]
    fn rejects_an_unknown_interval() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid interval")
            }
        }
        assert!(resolve_cask_downloads(&params("freetube", Some("bogus")), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"analytics": {"install": {"90d": {"freetube": 3000}}}}"#);
        assert!(resolve_cask_downloads(&params("freetube", None), &fetcher).is_err());
    }
}
