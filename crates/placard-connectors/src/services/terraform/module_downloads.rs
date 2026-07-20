use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn field_for_interval(interval: &str) -> Result<&'static str, String> {
    match interval {
        "dw" => Ok("week"),
        "dm" => Ok("month"),
        "dy" => Ok("year"),
        "dt" => Ok("total"),
        other => Err(format!(
            "'interval' parameter '{other}' is not one of dw, dm, dy, dt"
        )),
    }
}

pub(crate) fn resolve_module_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let namespace = params
        .get("namespace")
        .ok_or("terraform-module-downloads requires a data-namespace attribute")?;
    let name = params
        .get("name")
        .ok_or("terraform-module-downloads requires a data-name attribute")?;
    let provider = params
        .get("provider")
        .ok_or("terraform-module-downloads requires a data-provider attribute")?;
    let interval = params
        .get("interval")
        .ok_or("terraform-module-downloads requires a data-interval attribute")?;

    let namespace = validate_path_param("namespace", namespace)?;
    let name = validate_path_param("name", name)?;
    let provider = validate_path_param("provider", provider)?;
    let field = field_for_interval(interval)?;

    let url = format!(
        "https://registry.terraform.io/v2/modules/{namespace}/{name}/{provider}/downloads/summary"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "terraform response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let path = format!("data.attributes.{field}");
    let downloads = value
        .get(&path)
        .ok_or_else(|| format!("terraform response missing {path}"))?;
    downloads
        .as_text()
        .ok_or_else(|| format!("{field} was not a plain value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://registry.terraform.io/v2/modules/hashicorp/consul/aws/downloads/summary"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(interval: &str) -> HashMap<String, String> {
        HashMap::from([
            ("namespace".to_string(), "hashicorp".to_string()),
            ("name".to_string(), "consul".to_string()),
            ("provider".to_string(), "aws".to_string()),
            ("interval".to_string(), interval.to_string()),
        ])
    }

    #[test]
    fn extracts_the_field_matching_the_requested_interval() {
        let fetcher = FakeFetcher(
            r#"{"data": {"attributes": {"week": 12, "month": 340, "year": 5000, "total": 90000}}}"#,
        );
        assert_eq!(
            resolve_module_downloads(&params("dw"), &fetcher).unwrap(),
            "12"
        );
        assert_eq!(
            resolve_module_downloads(&params("dm"), &fetcher).unwrap(),
            "340"
        );
        assert_eq!(
            resolve_module_downloads(&params("dy"), &fetcher).unwrap(),
            "5000"
        );
        assert_eq!(
            resolve_module_downloads(&params("dt"), &fetcher).unwrap(),
            "90000"
        );
    }

    #[test]
    fn requires_all_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_module_downloads(&HashMap::new(), &Unused).is_err());
        let mut p = params("dw");
        p.remove("namespace");
        assert!(resolve_module_downloads(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_interval() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid interval")
            }
        }
        assert!(resolve_module_downloads(&params("weekly"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params("dw");
        p.insert("namespace".to_string(), "../etc".to_string());
        assert!(resolve_module_downloads(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"data": {"attributes": {}}}"#);
        assert!(resolve_module_downloads(&params("dw"), &fetcher).is_err());
    }
}
