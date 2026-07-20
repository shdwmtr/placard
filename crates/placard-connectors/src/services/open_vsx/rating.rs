use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn star_rating(rating: f64) -> String {
    let floored = rating.floor();
    let mut stars = String::new();
    while (stars.chars().count() as f64) < floored {
        stars.push('★');
    }
    let decimal = rating - floored;
    if decimal >= 0.875 {
        stars.push('★');
    } else if decimal >= 0.625 {
        stars.push('¾');
    } else if decimal >= 0.375 {
        stars.push('½');
    } else if decimal >= 0.125 {
        stars.push('¼');
    }
    while stars.chars().count() < 5 {
        stars.push('☆');
    }
    stars
}

pub(crate) fn resolve_rating(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let namespace = params
        .get("namespace")
        .ok_or("open-vsx-rating requires a data-namespace attribute")?;
    let namespace = validate_path_param("namespace", namespace)?;
    let extension = params
        .get("extension")
        .ok_or("open-vsx-rating requires a data-extension attribute")?;
    let extension = validate_path_param("extension", extension)?;
    let format = params.get("format").map(String::as_str).unwrap_or("rating");

    let url = format!("https://open-vsx.org/api/{namespace}/{extension}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "open-vsx response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let review_count = match value.get("reviewCount") {
        Some(Value::Number(n)) => *n as i64,
        _ => 0,
    };
    if review_count == 0 {
        return Ok("unrated".to_string());
    }
    let average = match value.get("averageRating") {
        Some(Value::Number(n)) => *n,
        _ => return Err("open-vsx response missing averageRating".to_string()),
    };

    Ok(if format == "stars" {
        star_rating(average)
    } else {
        format!("{average:.1}/5 ({review_count})")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://open-vsx.org/api/redhat/java");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(namespace: &str, extension: &str, format: Option<&str>) -> HashMap<String, String> {
        let mut m = HashMap::from([
            ("namespace".to_string(), namespace.to_string()),
            ("extension".to_string(), extension.to_string()),
        ]);
        if let Some(f) = format {
            m.insert("format".to_string(), f.to_string());
        }
        m
    }

    #[test]
    fn extracts_a_rating_message_by_default() {
        let fetcher = FakeFetcher(
            r#"{"version": "1.0.0", "timestamp": "2024-01-01T00:00:00Z", "averageRating": 4.5, "reviewCount": 250}"#,
        );
        let value = resolve_rating(&params("redhat", "java", None), &fetcher).unwrap();
        assert_eq!(value, "4.5/5 (250)");
    }

    #[test]
    fn renders_stars_when_format_is_stars() {
        let fetcher = FakeFetcher(
            r#"{"version": "1.0.0", "timestamp": "2024-01-01T00:00:00Z", "averageRating": 4.5, "reviewCount": 250}"#,
        );
        let value = resolve_rating(&params("redhat", "java", Some("stars")), &fetcher).unwrap();
        assert_eq!(value, "★★★★½");
    }

    #[test]
    fn returns_unrated_when_there_are_no_reviews() {
        let fetcher = FakeFetcher(
            r#"{"version": "1.0.0", "timestamp": "2024-01-01T00:00:00Z", "reviewCount": 0}"#,
        );
        let value = resolve_rating(&params("redhat", "java", None), &fetcher).unwrap();
        assert_eq!(value, "unrated");
    }

    #[test]
    fn requires_namespace_and_extension_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_rating(&HashMap::new(), &Unused).is_err());
        assert!(resolve_rating(&params("redhat", "", None), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_rating(&params("../etc", "java", None), &Unused).is_err());
    }

    #[test]
    fn errors_when_average_rating_is_missing_but_reviews_exist() {
        let fetcher = FakeFetcher(
            r#"{"version": "1.0.0", "timestamp": "2024-01-01T00:00:00Z", "reviewCount": 5}"#,
        );
        assert!(resolve_rating(&params("redhat", "java", None), &fetcher).is_err());
    }
}
