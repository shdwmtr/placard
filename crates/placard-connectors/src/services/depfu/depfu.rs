use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_depfu(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let vcs_type = params
        .get("vcs-type")
        .ok_or("depfu requires a data-vcs-type attribute")?;
    if vcs_type != "github" && vcs_type != "gitlab" {
        return Err("depfu data-vcs-type must be 'github' or 'gitlab'".to_string());
    }
    let user = params
        .get("user")
        .ok_or("depfu requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = params
        .get("repo")
        .ok_or("depfu requires a data-repo attribute")?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://depfu.com/{vcs_type}/shields/{user}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "depfu response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let text_field = value.get("text").ok_or("depfu response missing text")?;
    text_field
        .as_text()
        .ok_or_else(|| "text was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://depfu.com/github/shields/depfu/example-ruby");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(vcs_type: &str, user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("vcs-type".to_string(), vcs_type.to_string()),
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_text_from_a_depfu_shaped_response() {
        let fetcher = FakeFetcher(r#"{"text": "up to date", "colorscheme": "brightgreen"}"#);
        let value = resolve_depfu(&params("github", "depfu", "example-ruby"), &fetcher).unwrap();
        assert_eq!(value, "up to date");
    }

    #[test]
    fn requires_all_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_depfu(&HashMap::new(), &Unused).is_err());
        assert!(resolve_depfu(&params("svn", "depfu", "example-ruby"), &Unused).is_err());
        assert!(resolve_depfu(&params("github", "", "example-ruby"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_depfu(&params("github", "../etc", "example-ruby"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"colorscheme": "brightgreen"}"#);
        assert!(resolve_depfu(&params("github", "depfu", "example-ruby"), &fetcher).is_err());
    }
}
