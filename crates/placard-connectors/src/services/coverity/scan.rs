use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_scan(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_id = params
        .get("project-id")
        .ok_or("coverity-scan requires a data-project-id attribute")?;
    let project_id = validate_path_param("project-id", project_id)?;

    let url = format!("https://scan.coverity.com/projects/{project_id}/badge.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "coverity response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let message = value
        .get("message")
        .ok_or("coverity response missing message")?;
    message
        .as_text()
        .ok_or_else(|| "message was not a plain value".to_string())
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

    fn params(project_id: &str) -> HashMap<String, String> {
        HashMap::from([("project-id".to_string(), project_id.to_string())])
    }

    #[test]
    fn extracts_the_scan_message() {
        let fetcher = FakeFetcher {
            expected_url: "https://scan.coverity.com/projects/3997/badge.json",
            body: r#"{"message": "passed"}"#,
        };
        let value = resolve_scan(&params("3997"), &fetcher).unwrap();
        assert_eq!(value, "passed");
    }

    #[test]
    fn requires_project_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project id")
            }
        }
        assert!(resolve_scan(&HashMap::new(), &Unused).is_err());
        assert!(resolve_scan(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project id")
            }
        }
        assert!(resolve_scan(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://scan.coverity.com/projects/3997/badge.json",
            body: r#"{"other": 1}"#,
        };
        assert!(resolve_scan(&params("3997"), &fetcher).is_err());
    }
}
