use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_issues(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("bitbucket-issues requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("bitbucket-issues requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!(
        "https://bitbucket.org/api/2.0/repositories/{user}/{repo}/issues/?limit=0&q=%28state%20%3D%20%22new%22%20OR%20state%20%3D%20%22open%22%29"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "bitbucket response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let size = value.get("size").ok_or("bitbucket response missing size")?;
    size.as_text()
        .ok_or_else(|| "size was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://bitbucket.org/api/2.0/repositories/shields-io/test-repo/issues/?limit=0&q=%28state%20%3D%20%22new%22%20OR%20state%20%3D%20%22open%22%29"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_the_size_field_from_a_bitbucket_shaped_response() {
        let fetcher = FakeFetcher(r#"{"size": 7, "page": 1}"#);
        let value = resolve_issues(&params("shields-io", "test-repo"), &fetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_issues(&HashMap::new(), &Unused).is_err());
        assert!(resolve_issues(&params("shields-io", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_issues(&params("../etc", "test-repo"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_size_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"page": 1}"#);
        assert!(resolve_issues(&params("shields-io", "test-repo"), &fetcher).is_err());
    }
}
