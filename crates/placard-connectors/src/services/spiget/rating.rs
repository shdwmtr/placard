use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_rating(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let resource_id = params
        .get("resource-id")
        .ok_or("spiget-rating requires a data-resource-id attribute")?;
    let resource_id = validate_path_param("resource-id", resource_id)?;
    let format = params.get("format").map(String::as_str).unwrap_or("rating");

    let url = format!("https://api.spiget.org/v2/resources/{resource_id}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "spiget response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let average = match value.get("rating.average") {
        Some(Value::Number(n)) => *n,
        _ => return Err("spiget response missing rating.average".to_string()),
    };
    let count = match value.get("rating.count") {
        Some(Value::Number(n)) => *n,
        _ => return Err("spiget response missing rating.count".to_string()),
    };

    Ok(if format == "stars" {
        star_rating(average)
    } else {
        format!("{average:.2}/5 ({})", count as i64)
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.spiget.org/v2/resources/9089");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(resource_id: &str, format: Option<&str>) -> HashMap<String, String> {
        let mut m = HashMap::from([("resource-id".to_string(), resource_id.to_string())]);
        if let Some(f) = format {
            m.insert("format".to_string(), f.to_string());
        }
        m
    }

    #[test]
    fn extracts_a_rating_message_by_default() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 1, "file": {"type": "jar", "size": 1, "sizeUnit": "MB"}, "rating": {"count": 250, "average": 4.5}}"#,
        );
        let value = resolve_rating(&params("9089", None), &fetcher).unwrap();
        assert_eq!(value, "4.50/5 (250)");
    }

    #[test]
    fn renders_stars_when_format_is_stars() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 1, "file": {"type": "jar", "size": 1, "sizeUnit": "MB"}, "rating": {"count": 250, "average": 4.5}}"#,
        );
        let value = resolve_rating(&params("9089", Some("stars")), &fetcher).unwrap();
        assert_eq!(value, "★★★★½");
    }

    #[test]
    fn requires_resource_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_rating(&HashMap::new(), &Unused).is_err());
        assert!(resolve_rating(&params("", None), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_rating(&params("../etc", None), &Unused).is_err());
    }

    #[test]
    fn errors_when_rating_field_is_missing() {
        let fetcher = FakeFetcher(
            r#"{"downloads": 1, "file": {"type": "jar", "size": 1, "sizeUnit": "MB"}}"#,
        );
        assert!(resolve_rating(&params("9089", None), &fetcher).is_err());
    }
}
