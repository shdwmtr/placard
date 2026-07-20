use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn fetch_script(script_id: &str, fetcher: &dyn Fetcher) -> Result<json::Value, String> {
    let primary = format!("https://greasyfork.org/scripts/{script_id}.json");
    let text = match fetcher.fetch(&primary) {
        Ok(bytes) => String::from_utf8(bytes)
            .map_err(|_| "greasyfork response was not valid UTF-8".to_string())?,
        Err(_) => {
            let fallback = format!("https://sleazyfork.org/scripts/{script_id}.json");
            let bytes = fetcher.fetch(&fallback)?;
            String::from_utf8(bytes)
                .map_err(|_| "sleazyfork response was not valid UTF-8".to_string())?
        }
    };
    json::parse(&text)
}

pub(crate) fn resolve_rating(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let script_id = params
        .get("script-id")
        .ok_or("greasyfork-rating requires a data-script-id attribute")?;
    let script_id = validate_path_param("script-id", script_id)?;

    let value = fetch_script(script_id, fetcher)?;
    let good = value
        .get("good_ratings")
        .and_then(|v| v.as_text())
        .ok_or("greasyfork response missing good_ratings")?;
    let ok = value
        .get("ok_ratings")
        .and_then(|v| v.as_text())
        .ok_or("greasyfork response missing ok_ratings")?;
    let bad = value
        .get("bad_ratings")
        .and_then(|v| v.as_text())
        .ok_or("greasyfork response missing bad_ratings")?;
    Ok(format!("{good} good, {ok} ok, {bad} bad"))
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

    fn params(script_id: &str) -> HashMap<String, String> {
        HashMap::from([("script-id".to_string(), script_id.to_string())])
    }

    #[test]
    fn combines_the_rating_breakdown() {
        let fetcher = FakeFetcher {
            expected_url: "https://greasyfork.org/scripts/406540.json",
            body: r#"{"good_ratings": 120, "ok_ratings": 5, "bad_ratings": 2}"#,
        };
        let value = resolve_rating(&params("406540"), &fetcher).unwrap();
        assert_eq!(value, "120 good, 5 ok, 2 bad");
    }

    #[test]
    fn falls_back_to_sleazyfork_when_greasyfork_fetch_fails() {
        struct FallbackFetcher;
        impl Fetcher for FallbackFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                if url == "https://greasyfork.org/scripts/406540.json" {
                    return Err("not found".to_string());
                }
                assert_eq!(url, "https://sleazyfork.org/scripts/406540.json");
                Ok(br#"{"good_ratings": 1, "ok_ratings": 2, "bad_ratings": 3}"#.to_vec())
            }
        }
        let value = resolve_rating(&params("406540"), &FallbackFetcher).unwrap();
        assert_eq!(value, "1 good, 2 ok, 3 bad");
    }

    #[test]
    fn requires_script_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_rating(&HashMap::new(), &Unused).is_err());
        assert!(resolve_rating(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_rating(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_a_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://greasyfork.org/scripts/406540.json",
            body: r#"{"good_ratings": 1, "ok_ratings": 2}"#,
        };
        assert!(resolve_rating(&params("406540"), &fetcher).is_err());
    }
}
