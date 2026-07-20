use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_likes(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("pub-likes requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://pub.dev/api/packages/{package}/score");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "pub response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("likeCount")
        .ok_or("pub response missing likeCount")?;
    count
        .as_text()
        .ok_or_else(|| "likeCount was not a plain value".to_string())
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
    fn extracts_like_count_from_a_pub_score_response() {
        let fetcher = FakeFetcher(r#"{"grantedPoints": 130, "likeCount": 512}"#);
        let value = resolve_likes(&params("analysis_options"), &fetcher).unwrap();
        assert_eq!(value, "512");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_likes(&HashMap::new(), &Unused).is_err());
        assert!(resolve_likes(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_likes(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"grantedPoints": 130}"#);
        assert!(resolve_likes(&params("analysis_options"), &fetcher).is_err());
    }
}
