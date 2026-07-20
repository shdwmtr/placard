use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_pull_request(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let provider = params
        .get("provider")
        .ok_or("coderabbit-pull-request requires a data-provider attribute")?;
    if !matches!(provider.as_str(), "github" | "bitbucket" | "gitlab") {
        return Err(format!(
            "'provider' parameter '{provider}' is not one of github, bitbucket, gitlab"
        ));
    }
    let org = params
        .get("org")
        .ok_or("coderabbit-pull-request requires a data-org attribute")?;
    let repo = params
        .get("repo")
        .ok_or("coderabbit-pull-request requires a data-repo attribute")?;
    let org = validate_path_param("org", org)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://api.coderabbit.ai/stats/{provider}/{org}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "coderabbit response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let reviews = value
        .get("reviews")
        .ok_or("coderabbit response missing reviews")?;
    reviews
        .as_text()
        .ok_or_else(|| "reviews was not a plain value".to_string())
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

    fn params(provider: &str, org: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("provider".to_string(), provider.to_string()),
            ("org".to_string(), org.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_review_count() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.coderabbit.ai/stats/github/coderabbitai/ast-grep-essentials",
            body: r#"{"reviews": 42}"#,
        };
        let value = resolve_pull_request(
            &params("github", "coderabbitai", "ast-grep-essentials"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_provider_org_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_pull_request(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_pull_request(
                &params("svn", "coderabbitai", "ast-grep-essentials"),
                &Unused
            )
            .is_err()
        );
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
            resolve_pull_request(&params("github", "../etc", "ast-grep-essentials"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.coderabbit.ai/stats/github/coderabbitai/ast-grep-essentials",
            body: r#"{"other": 1}"#,
        };
        assert!(
            resolve_pull_request(
                &params("github", "coderabbitai", "ast-grep-essentials"),
                &fetcher
            )
            .is_err()
        );
    }
}
