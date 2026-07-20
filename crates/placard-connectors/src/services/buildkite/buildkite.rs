use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_buildkite(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let identifier = params
        .get("identifier")
        .ok_or("buildkite requires a data-identifier attribute")?;
    let identifier = validate_path_param("identifier", identifier)?;

    let mut url = format!("https://badge.buildkite.com/{identifier}.json");
    if let Some(branch) = params.get("branch") {
        let branch = validate_path_param("branch", branch)?;
        url.push_str(&format!("?branch={branch}"));
    }

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "buildkite response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let status = value
        .get("status")
        .ok_or("buildkite response missing status")?;
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

    fn params(identifier: &str) -> HashMap<String, String> {
        HashMap::from([("identifier".to_string(), identifier.to_string())])
    }

    #[test]
    fn extracts_status_from_a_buildkite_shaped_response() {
        let fetcher = FakeFetcher(
            "https://badge.buildkite.com/3826789cf8890b426057e6fe1c4e683bdf04fa24d498885489.json",
            r#"{"status": "passed"}"#,
        );
        let value = resolve_buildkite(
            &params("3826789cf8890b426057e6fe1c4e683bdf04fa24d498885489"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "passed");
    }

    #[test]
    fn appends_branch_when_provided() {
        let mut p = params("3826789cf8890b426057e6fe1c4e683bdf04fa24d498885489");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher(
            "https://badge.buildkite.com/3826789cf8890b426057e6fe1c4e683bdf04fa24d498885489.json?branch=master",
            r#"{"status": "passed"}"#,
        );
        let value = resolve_buildkite(&p, &fetcher).unwrap();
        assert_eq!(value, "passed");
    }

    #[test]
    fn requires_identifier_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_buildkite(&HashMap::new(), &Unused).is_err());
        assert!(resolve_buildkite(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_buildkite(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_status_is_missing() {
        let fetcher = FakeFetcher(
            "https://badge.buildkite.com/3826789cf8890b426057e6fe1c4e683bdf04fa24d498885489.json",
            r#"{}"#,
        );
        assert!(
            resolve_buildkite(
                &params("3826789cf8890b426057e6fe1c4e683bdf04fa24d498885489"),
                &fetcher
            )
            .is_err()
        );
    }
}
