use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_stars(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let slug = params
        .get("slug")
        .ok_or("hangar-stars requires a data-slug attribute")?;
    let slug = validate_path_param("slug", slug)?;

    let url = format!("https://hangar.papermc.io/api/v1/projects/{slug}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "hangar response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let stars = value
        .get("stats.stars")
        .ok_or("hangar response missing stats.stars")?;
    stars
        .as_text()
        .ok_or_else(|| "stats.stars was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://hangar.papermc.io/api/v1/projects/Essentials");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(slug: &str) -> HashMap<String, String> {
        HashMap::from([("slug".to_string(), slug.to_string())])
    }

    #[test]
    fn extracts_stars_from_a_hangar_shaped_response() {
        let fetcher = FakeFetcher(
            r#"{"stats": {"views": 1, "downloads": 2, "recentViews": 3, "recentDownloads": 4, "stars": 42, "watchers": 5}}"#,
        );
        let value = resolve_stars(&params("Essentials"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_slug_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_stars(&HashMap::new(), &Unused).is_err());
        assert!(resolve_stars(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_stars(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"stats": {"views": 1}}"#);
        assert!(resolve_stars(&params("Essentials"), &fetcher).is_err());
    }
}
