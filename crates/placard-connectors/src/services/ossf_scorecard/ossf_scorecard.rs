use crate::Fetcher;
use crate::json;
use crate::services::validate_path_param;
use std::collections::HashMap;

pub(crate) fn resolve_ossf_scorecard(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let host = params
        .get("host")
        .ok_or("ossf-scorecard requires a data-host attribute")?;
    let org_name = params
        .get("org-name")
        .ok_or("ossf-scorecard requires a data-org-name attribute")?;
    let repo_name = params
        .get("repo-name")
        .ok_or("ossf-scorecard requires a data-repo-name attribute")?;
    let host = validate_path_param("host", host)?;
    let org_name = validate_path_param("org-name", org_name)?;
    let repo_name = validate_path_param("repo-name", repo_name)?;

    let url = format!("https://api.securityscorecards.dev/projects/{host}/{org_name}/{repo_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "ossf-scorecard response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let score = value
        .get("score")
        .ok_or("ossf-scorecard response missing score")?;
    score
        .as_text()
        .ok_or_else(|| "score was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.securityscorecards.dev/projects/github.com/rohankh532/org-workflow-add"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(host: &str, org_name: &str, repo_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("host".to_string(), host.to_string()),
            ("org-name".to_string(), org_name.to_string()),
            ("repo-name".to_string(), repo_name.to_string()),
        ])
    }

    #[test]
    fn extracts_the_score() {
        let fetcher = FakeFetcher(r#"{"date": "2023-01-01", "score": 7.8, "repo": {}}"#);
        let value = resolve_ossf_scorecard(
            &params("github.com", "rohankh532", "org-workflow-add"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "7.8");
    }

    #[test]
    fn requires_host_org_name_and_repo_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_ossf_scorecard(&HashMap::new(), &Unused).is_err());
        assert!(resolve_ossf_scorecard(&params("github.com", "rohankh532", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(
            resolve_ossf_scorecard(&params("github.com", "../etc", "org-workflow-add"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"date": "2023-01-01"}"#);
        assert!(
            resolve_ossf_scorecard(
                &params("github.com", "rohankh532", "org-workflow-add"),
                &fetcher
            )
            .is_err()
        );
    }
}
