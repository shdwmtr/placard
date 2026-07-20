use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_rating(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let plugin_id = params
        .get("plugin-id")
        .ok_or("jetbrains-rating requires a data-plugin-id attribute")?;
    let plugin_id = validate_path_param("plugin-id", plugin_id)?;
    let format = params.get("format").map(String::as_str).unwrap_or("rating");

    let url = format!("https://plugins.jetbrains.com/api/plugins/{plugin_id}/rating");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "jetbrains response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let votes = match value.get("votes") {
        Some(Value::Object(fields)) => fields,
        _ => return Err("jetbrains response missing votes".to_string()),
    };
    let mean_rating = match value.get("meanRating") {
        Some(Value::Number(n)) => *n,
        _ => return Err("jetbrains response missing meanRating".to_string()),
    };

    let mut vote_sum = 0.0;
    let mut vote_count = 0.0;
    for (rating, count) in votes {
        let count = match count {
            Value::Number(n) => *n,
            _ => continue,
        };
        let rating: f64 = rating.parse().unwrap_or(0.0);
        vote_sum += rating * count;
        vote_count += count;
    }

    if vote_count == 0.0 {
        return Err("jetbrains plugin has no ratings".to_string());
    }

    let rating = (vote_sum + 2.0 * mean_rating) / (vote_count + 2.0);

    Ok(if format == "stars" {
        star_rating(rating)
    } else {
        format!("{rating:.1}/5")
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
            assert_eq!(
                url,
                "https://plugins.jetbrains.com/api/plugins/11941/rating"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(plugin_id: &str, format: Option<&str>) -> HashMap<String, String> {
        let mut m = HashMap::from([("plugin-id".to_string(), plugin_id.to_string())]);
        if let Some(f) = format {
            m.insert("format".to_string(), f.to_string());
        }
        m
    }

    #[test]
    fn extracts_a_rating_message_by_default() {
        let fetcher = FakeFetcher(
            r#"{"votes": {"5": 100, "4": 20, "3": 5}, "meanVotes": 125, "meanRating": 4.6}"#,
        );
        let value = resolve_rating(&params("11941", None), &fetcher).unwrap();
        assert_eq!(value, "4.8/5");
    }

    #[test]
    fn renders_stars_when_format_is_stars() {
        let fetcher = FakeFetcher(r#"{"votes": {"5": 100}, "meanVotes": 100, "meanRating": 5.0}"#);
        let value = resolve_rating(&params("11941", Some("stars")), &fetcher).unwrap();
        assert_eq!(value, "★★★★★");
    }

    #[test]
    fn requires_plugin_id_param() {
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
    fn errors_when_votes_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"meanVotes": 0, "meanRating": 0}"#);
        assert!(resolve_rating(&params("11941", None), &fetcher).is_err());
    }

    #[test]
    fn errors_when_there_are_no_votes() {
        let fetcher = FakeFetcher(r#"{"votes": {}, "meanVotes": 0, "meanRating": 0}"#);
        assert!(resolve_rating(&params("11941", None), &fetcher).is_err());
    }
}
