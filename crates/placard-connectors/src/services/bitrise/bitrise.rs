use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_bitrise(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let app_id = params
        .get("app-id")
        .ok_or("bitrise requires a data-app-id attribute")?;
    let token = params
        .get("token")
        .ok_or("bitrise requires a data-token attribute")?;
    let app_id = validate_path_param("app-id", app_id)?;
    let token = validate_path_param("token", token)?;

    let mut url = format!("https://app.bitrise.io/app/{app_id}/status.json?token={token}");
    if let Some(branch) = params.get("branch") {
        let branch = validate_path_param("branch", branch)?;
        url.push_str(&format!("&branch={branch}"));
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "bitrise response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let status = value
        .get("status")
        .ok_or("bitrise response missing status")?;
    status
        .as_text()
        .ok_or_else(|| "status was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str, &'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.0);
            Ok(self.1.as_bytes().to_vec())
        }
    }

    fn params(app_id: &str, token: &str) -> HashMap<String, String> {
        HashMap::from([
            ("app-id".to_string(), app_id.to_string()),
            ("token".to_string(), token.to_string()),
        ])
    }

    #[test]
    fn extracts_status_from_a_bitrise_shaped_response() {
        let fetcher = FakeFetcher(
            "https://app.bitrise.io/app/9fa2e96dc9458fbb/status.json?token=abc123def456",
            r#"{"status": "success"}"#,
        );
        let value = resolve_bitrise(&params("9fa2e96dc9458fbb", "abc123def456"), &fetcher).unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn appends_branch_when_provided() {
        let mut p = params("9fa2e96dc9458fbb", "abc123def456");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher(
            "https://app.bitrise.io/app/9fa2e96dc9458fbb/status.json?token=abc123def456&branch=master",
            r#"{"status": "success"}"#,
        );
        let value = resolve_bitrise(&p, &fetcher).unwrap();
        assert_eq!(value, "success");
    }

    #[test]
    fn requires_app_id_and_token_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_bitrise(&HashMap::new(), &Unused).is_err());
        assert!(resolve_bitrise(&params("9fa2e96dc9458fbb", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_bitrise(&params("../etc/passwd", "abc123def456"), &Unused).is_err());
    }

    #[test]
    fn errors_when_status_is_missing() {
        let fetcher = FakeFetcher(
            "https://app.bitrise.io/app/9fa2e96dc9458fbb/status.json?token=abc123def456",
            r#"{}"#,
        );
        assert!(resolve_bitrise(&params("9fa2e96dc9458fbb", "abc123def456"), &fetcher).is_err());
    }
}
