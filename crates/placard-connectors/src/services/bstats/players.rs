use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_players(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let pluginid = params
        .get("pluginid")
        .ok_or("bstats-players requires a data-pluginid attribute")?;
    let pluginid = validate_path_param("pluginid", pluginid)?;

    let url =
        format!("https://bstats.org/api/v1/plugins/{pluginid}/charts/players/data?maxElements=1");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "bstats response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let Value::Array(rows) = value else {
        return Err("bstats response was not an array".to_string());
    };
    let first = rows.into_iter().next().ok_or("bstats response was empty")?;
    let Value::Array(pair) = first else {
        return Err("bstats response entry was not an array".to_string());
    };
    let count = pair
        .get(1)
        .ok_or("bstats response entry missing second value")?;
    count
        .as_text()
        .ok_or_else(|| "bstats players value was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://bstats.org/api/v1/plugins/1/charts/players/data?maxElements=1"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(pluginid: &str) -> HashMap<String, String> {
        HashMap::from([("pluginid".to_string(), pluginid.to_string())])
    }

    #[test]
    fn extracts_the_latest_player_count() {
        let fetcher = FakeFetcher(r#"[[1690000000000, 4213]]"#);
        let value = resolve_players(&params("1"), &fetcher).unwrap();
        assert_eq!(value, "4213");
    }

    #[test]
    fn requires_pluginid_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid pluginid")
            }
        }
        assert!(resolve_players(&HashMap::new(), &Unused).is_err());
        assert!(resolve_players(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid pluginid")
            }
        }
        assert!(resolve_players(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_response_is_empty() {
        let fetcher = FakeFetcher(r#"[]"#);
        assert!(resolve_players(&params("1"), &fetcher).is_err());
    }

    #[test]
    fn errors_on_a_malformed_response() {
        let fetcher = FakeFetcher(r#"{"not": "an array"}"#);
        assert!(resolve_players(&params("1"), &fetcher).is_err());
    }
}
