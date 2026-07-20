use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn field_name(metric: &str) -> Result<&'static str, String> {
    match metric {
        "keys" => Ok("Keys"),
        "clicks" => Ok("Clicks"),
        "uptime" => Ok("UptimeSeconds"),
        "download" => Ok("Download"),
        "upload" => Ok("Upload"),
        other => Err(format!(
            "'metric' parameter '{other}' is not one of keys, clicks, uptime, download, upload"
        )),
    }
}

fn rank_field_name(metric: &str) -> Result<&'static str, String> {
    match metric {
        "keys" => Ok("Ranks.Keys"),
        "clicks" => Ok("Ranks.Clicks"),
        "uptime" => Ok("Ranks.Uptime"),
        "download" => Ok("Ranks.Download"),
        "upload" => Ok("Ranks.Upload"),
        other => Err(format!(
            "'metric' parameter '{other}' is not one of keys, clicks, uptime, download, upload"
        )),
    }
}

pub(crate) fn resolve_whatpulse(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let metric = params
        .get("metric")
        .ok_or("whatpulse requires a data-metric attribute")?;
    let user_type = params
        .get("user-type")
        .ok_or("whatpulse requires a data-user-type attribute")?;
    let id = params
        .get("id")
        .ok_or("whatpulse requires a data-id attribute")?;

    if user_type != "user" && user_type != "team" {
        return Err(format!(
            "'user-type' parameter '{user_type}' is not one of user, team"
        ));
    }
    let id = validate_path_param("id", id)?;

    let field = if params.contains_key("rank") {
        rank_field_name(metric)?
    } else {
        field_name(metric)?
    };

    let url = format!("https://api.whatpulse.org/{user_type}.php?{user_type}={id}&format=json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "whatpulse response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let result = value
        .get(field)
        .ok_or_else(|| format!("whatpulse response missing {field}"))?;
    result
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
                "https://api.whatpulse.org/user.php?user=179734&format=json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(metric: &str, user_type: &str, id: &str) -> HashMap<String, String> {
        HashMap::from([
            ("metric".to_string(), metric.to_string()),
            ("user-type".to_string(), user_type.to_string()),
            ("id".to_string(), id.to_string()),
        ])
    }

    #[test]
    fn extracts_the_raw_metric_field() {
        let fetcher = FakeFetcher(
            r#"{"Keys": 1234567, "Clicks": 89012, "UptimeSeconds": 500000, "Download": "1.2GB", "Upload": "300MB"}"#,
        );
        let value = resolve_whatpulse(&params("keys", "user", "179734"), &fetcher).unwrap();
        assert_eq!(value, "1234567");
    }

    #[test]
    fn extracts_a_string_valued_field() {
        let fetcher = FakeFetcher(
            r#"{"Keys": 1234567, "Clicks": 89012, "UptimeSeconds": 500000, "Download": "1.2GB", "Upload": "300MB"}"#,
        );
        let value = resolve_whatpulse(&params("download", "user", "179734"), &fetcher).unwrap();
        assert_eq!(value, "1.2GB");
    }

    #[test]
    fn extracts_rank_when_rank_param_present() {
        struct RankFetcher;
        impl Fetcher for RankFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://api.whatpulse.org/team.php?team=1295&format=json"
                );
                Ok(br#"{"Keys": 1, "Clicks": 2, "UptimeSeconds": 3, "Download": "d", "Upload": "u",
                    "Ranks": {"Keys": "42", "Clicks": "1", "Download": "5", "Upload": "6", "Uptime": "7"}}"#
                    .to_vec())
            }
        }
        let mut p = params("keys", "team", "1295");
        p.insert("rank".to_string(), String::new());
        let value = resolve_whatpulse(&p, &RankFetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_metric_user_type_and_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_whatpulse(&HashMap::new(), &Unused).is_err());
        assert!(resolve_whatpulse(&params("keys", "user", ""), &Unused).is_err());
        assert!(resolve_whatpulse(&params("bogus", "user", "179734"), &Unused).is_err());
        assert!(resolve_whatpulse(&params("keys", "bogus", "179734"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid id")
            }
        }
        assert!(resolve_whatpulse(&params("keys", "user", "../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"Keys": 1}"#);
        assert!(resolve_whatpulse(&params("uptime", "user", "179734"), &fetcher).is_err());
    }
}
