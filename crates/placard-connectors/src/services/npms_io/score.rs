use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn percent_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn score_type_for(kind: &str) -> Result<&'static str, String> {
    match kind {
        "final-score" => Ok("final"),
        "maintenance-score" => Ok("maintenance"),
        "popularity-score" => Ok("popularity"),
        "quality-score" => Ok("quality"),
        other => Err(format!("unknown npms-io-score type '{other}'")),
    }
}

pub(crate) fn resolve_score(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("npms-io-score requires a data-package attribute")?;
    if package.is_empty() {
        return Err("'package' parameter must not be empty".to_string());
    }
    let kind = params
        .get("type")
        .ok_or("npms-io-score requires a data-type attribute")?;
    let score_type = score_type_for(kind)?;

    let url = format!(
        "https://api.npms.io/v2/package/{}",
        percent_encode_component(package)
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "npms.io response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;

    let path = if score_type == "final" {
        "score.final".to_string()
    } else {
        format!("score.detail.{score_type}")
    };
    let value = doc
        .get(&path)
        .ok_or_else(|| format!("npms.io response missing {path}"))?;
    let text = value
        .as_text()
        .ok_or_else(|| format!("{path} was not a plain value"))?;
    let score: f64 = text
        .parse()
        .map_err(|_| format!("{path} was not numeric"))?;

    Ok(format!("{:.0}%", score * 100.0))
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

    fn params(package: &str, kind: &str) -> HashMap<String, String> {
        HashMap::from([
            ("package".to_string(), package.to_string()),
            ("type".to_string(), kind.to_string()),
        ])
    }

    #[test]
    fn extracts_the_final_score() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.npms.io/v2/package/command",
            body: r#"{"score": {"final": 0.85, "detail": {"maintenance": 0.9, "popularity": 0.7, "quality": 0.95}}}"#,
        };
        let value = resolve_score(&params("command", "final-score"), &fetcher).unwrap();
        assert_eq!(value, "85%");
    }

    #[test]
    fn extracts_a_detail_score() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.npms.io/v2/package/command",
            body: r#"{"score": {"final": 0.85, "detail": {"maintenance": 0.9, "popularity": 0.7, "quality": 0.95}}}"#,
        };
        let value = resolve_score(&params("command", "quality-score"), &fetcher).unwrap();
        assert_eq!(value, "95%");
    }

    #[test]
    fn percent_encodes_scoped_package_names() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.npms.io/v2/package/%40vue%2Fcli",
            body: r#"{"score": {"final": 0.5, "detail": {"maintenance": 0.5, "popularity": 0.5, "quality": 0.5}}}"#,
        };
        let value = resolve_score(&params("@vue/cli", "final-score"), &fetcher).unwrap();
        assert_eq!(value, "50%");
    }

    #[test]
    fn requires_package_and_type_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_score(&HashMap::new(), &Unused).is_err());
        assert!(resolve_score(&params("", "final-score"), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_types() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an unknown type")
            }
        }
        assert!(resolve_score(&params("command", "bogus-score"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.npms.io/v2/package/command",
            body: r#"{"score": {}}"#,
        };
        assert!(resolve_score(&params("command", "final-score"), &fetcher).is_err());
    }
}
