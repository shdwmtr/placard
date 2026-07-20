use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_points(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pub-points requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://pub.dev/api/packages/{package}/score");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "pub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let granted = value
        .get("grantedPoints")
        .ok_or("pub response missing grantedPoints")?
        .as_text()
        .ok_or_else(|| "grantedPoints was not a plain value".to_string())?;
    let max = value
        .get("maxPoints")
        .ok_or("pub response missing maxPoints")?
        .as_text()
        .ok_or_else(|| "maxPoints was not a plain value".to_string())?;
    Ok(format!("{granted}/{max}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://pub.dev/api/packages/analysis_options/score");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn formats_granted_over_max_points_from_a_pub_score_response() {
        let fetcher = FakeFetcher(r#"{"grantedPoints": 130, "maxPoints": 140}"#);
        let value = resolve_points(&params("analysis_options"), &fetcher).unwrap();
        assert_eq!(value, "130/140");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_points(&HashMap::new(), &Unused).is_err());
        assert!(resolve_points(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_points(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_a_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"grantedPoints": 130}"#);
        assert!(resolve_points(&params("analysis_options"), &fetcher).is_err());
    }
}
