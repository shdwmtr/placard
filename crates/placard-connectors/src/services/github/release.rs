use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_release(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-release requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-release requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let display_name = params
        .get("display_name")
        .map(String::as_str)
        .unwrap_or("tag");
    if display_name != "tag" && display_name != "release" {
        return Err(format!(
            "'display_name' parameter '{display_name}' is not one of tag, release"
        ));
    }

    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let tag_name = value.get("tag_name").and_then(|v| v.as_text());

    if display_name == "tag" {
        return tag_name.ok_or_else(|| "github response missing tag_name".to_string());
    }

    value
        .get("name")
        .and_then(|v| v.as_text())
        .filter(|s| !s.is_empty())
        .or(tag_name)
        .ok_or_else(|| "github response missing name and tag_name".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/expressjs/express/releases/latest"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "expressjs".to_string()),
            ("repo".to_string(), "express".to_string()),
        ])
    }

    #[test]
    fn extracts_the_tag_name_by_default() {
        let fetcher =
            FakeFetcher(r#"{"tag_name": "4.18.2", "name": "4.18.2", "prerelease": false}"#);
        let value = resolve_release(&params(), &fetcher).unwrap();
        assert_eq!(value, "4.18.2");
    }

    #[test]
    fn extracts_the_release_name_when_requested() {
        let fetcher =
            FakeFetcher(r#"{"tag_name": "4.18.2", "name": "Express 4.18.2", "prerelease": false}"#);
        let mut p = params();
        p.insert("display_name".to_string(), "release".to_string());
        let value = resolve_release(&p, &fetcher).unwrap();
        assert_eq!(value, "Express 4.18.2");
    }

    #[test]
    fn falls_back_to_tag_name_when_release_name_is_empty() {
        let fetcher = FakeFetcher(r#"{"tag_name": "4.18.2", "name": "", "prerelease": false}"#);
        let mut p = params();
        p.insert("display_name".to_string(), "release".to_string());
        let value = resolve_release(&p, &fetcher).unwrap();
        assert_eq!(value, "4.18.2");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_release(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("repo".to_string(), String::new());
        assert!(resolve_release(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params();
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_release(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_tag_name_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "Express", "prerelease": false}"#);
        assert!(resolve_release(&params(), &fetcher).is_err());
    }
}
