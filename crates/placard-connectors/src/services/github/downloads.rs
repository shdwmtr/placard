use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-downloads requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-downloads requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url = match params.get("tag") {
        Some(tag) if tag != "latest" => {
            let tag = validate_path_param("tag", tag)?;
            format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}")
        }
        _ => format!("https://api.github.com/repos/{owner}/{repo}/releases/latest"),
    };

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let assets = match value.get("assets") {
        Some(Value::Array(items)) => items,
        _ => return Err("github response missing assets array".to_string()),
    };

    let mut total = 0i64;
    for asset in assets {
        let count = asset
            .get("download_count")
            .and_then(Value::as_text)
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or("asset missing download_count")?;
        total += count;
    }
    Ok(total.to_string())
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

    fn params(owner: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn sums_download_counts_across_assets_for_the_latest_release() {
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases/latest",
            r#"{"assets": [{"name": "a.deb", "download_count": 100}, {"name": "b.rpm", "download_count": 42}]}"#,
        );
        let value = resolve_downloads(&params("atom", "atom"), &fetcher).unwrap();
        assert_eq!(value, "142");
    }

    #[test]
    fn uses_the_tags_endpoint_when_a_specific_tag_is_given() {
        let mut p = params("atom", "atom");
        p.insert("tag".to_string(), "v1.0.0".to_string());
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases/tags/v1.0.0",
            r#"{"assets": [{"name": "a.deb", "download_count": 7}]}"#,
        );
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("atom", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc", "atom"), &Unused).is_err());
    }

    #[test]
    fn errors_when_assets_field_is_missing() {
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases/latest",
            r#"{"id": 1}"#,
        );
        assert!(resolve_downloads(&params("atom", "atom"), &fetcher).is_err());
    }
}
