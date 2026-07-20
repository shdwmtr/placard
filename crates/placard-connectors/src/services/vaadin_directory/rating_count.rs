use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_rating_count(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("vaadin-directory-rating-count requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!(
        "https://vaadin.com/vaadincom/directory-service/components/search/findByUrlIdentifier?projection=summary&urlIdentifier={package_name}"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "vaadin directory response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let rating_count = value
        .get("ratingCount")
        .ok_or("vaadin directory response missing ratingCount")?;
    rating_count
        .as_text()
        .ok_or_else(|| "ratingCount was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://vaadin.com/vaadincom/directory-service/components/search/findByUrlIdentifier?projection=summary&urlIdentifier=vaadinvaadin-grid"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_rating_count_from_a_vaadin_directory_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"ratingCount": 42, "averageRating": 4.5, "status": "PUBLISHED", "latestAvailableRelease": {"name": "1.0.0", "publicationDate": "2020-01-01T00:00:00Z"}}"#,
        );
        let value = resolve_rating_count(&params("vaadinvaadin-grid"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_rating_count(&HashMap::new(), &Unused).is_err());
        assert!(resolve_rating_count(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_rating_count(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"averageRating": 4.5}"#);
        assert!(resolve_rating_count(&params("vaadinvaadin-grid"), &fetcher).is_err());
    }
}
