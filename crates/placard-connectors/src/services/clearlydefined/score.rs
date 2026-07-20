use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_score(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let type_ = params
        .get("type")
        .ok_or("clearlydefined-score requires a data-type attribute")?;
    let provider = params
        .get("provider")
        .ok_or("clearlydefined-score requires a data-provider attribute")?;
    let namespace = params
        .get("namespace")
        .ok_or("clearlydefined-score requires a data-namespace attribute")?;
    let name = params
        .get("name")
        .ok_or("clearlydefined-score requires a data-name attribute")?;
    let revision = params
        .get("revision")
        .ok_or("clearlydefined-score requires a data-revision attribute")?;
    let type_ = validate_path_param("type", type_)?;
    let provider = validate_path_param("provider", provider)?;
    let namespace = validate_path_param("namespace", namespace)?;
    let name = validate_path_param("name", name)?;
    let revision = validate_path_param("revision", revision)?;

    let url = format!(
        "https://api.clearlydefined.io/definitions/{type_}/{provider}/{namespace}/{name}/{revision}"
    );
    let bytes = fetcher.fetch(&url)?;
    if bytes.is_empty() {
        return Err("unknown type, provider, or upstream issue".to_string());
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| "clearlydefined response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let score = value
        .get("scores.effective")
        .ok_or("clearlydefined response missing scores.effective")?;
    score
        .as_text()
        .ok_or_else(|| "scores.effective was not a plain value".to_string())
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

    fn params(
        type_: &str,
        provider: &str,
        namespace: &str,
        name: &str,
        revision: &str,
    ) -> HashMap<String, String> {
        HashMap::from([
            ("type".to_string(), type_.to_string()),
            ("provider".to_string(), provider.to_string()),
            ("namespace".to_string(), namespace.to_string()),
            ("name".to_string(), name.to_string()),
            ("revision".to_string(), revision.to_string()),
        ])
    }

    #[test]
    fn extracts_the_effective_score() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.clearlydefined.io/definitions/npm/npmjs/-/jquery/3.4.1",
            body: r#"{"scores": {"effective": 87}, "described": {"files": 12}}"#,
        };
        let value =
            resolve_score(&params("npm", "npmjs", "-", "jquery", "3.4.1"), &fetcher).unwrap();
        assert_eq!(value, "87");
    }

    #[test]
    fn requires_all_path_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_score(&HashMap::new(), &Unused).is_err());
        assert!(resolve_score(&params("npm", "npmjs", "-", "jquery", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_score(&params("npm", "../etc", "-", "jquery", "3.4.1"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_response_body_is_empty() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.clearlydefined.io/definitions/npm/npmjs/-/jquery/3.4.1",
            body: "",
        };
        assert!(resolve_score(&params("npm", "npmjs", "-", "jquery", "3.4.1"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.clearlydefined.io/definitions/npm/npmjs/-/jquery/3.4.1",
            body: r#"{"described": {"files": 12}}"#,
        };
        assert!(resolve_score(&params("npm", "npmjs", "-", "jquery", "3.4.1"), &fetcher).is_err());
    }
}
