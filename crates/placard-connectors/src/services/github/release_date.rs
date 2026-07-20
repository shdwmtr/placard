use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_release_date(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-release-date requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-release-date requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let variant = params
        .get("variant")
        .map(String::as_str)
        .unwrap_or("release-date");
    if variant != "release-date" && variant != "release-date-pre" {
        return Err(format!(
            "'variant' parameter '{variant}' is not one of release-date, release-date-pre"
        ));
    }

    let display_date = params
        .get("display_date")
        .map(String::as_str)
        .unwrap_or("published_at");
    if display_date != "created_at" && display_date != "published_at" {
        return Err(format!(
            "'display_date' parameter '{display_date}' is not one of created_at, published_at"
        ));
    }

    let url = if variant == "release-date" {
        format!("https://api.github.com/repos/{owner}/{repo}/releases/latest")
    } else {
        format!("https://api.github.com/repos/{owner}/{repo}/releases")
    };
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let release = if let Value::Array(items) = &value {
        items.first().ok_or("github response had no releases")?
    } else {
        &value
    };

    release
        .get(display_date)
        .and_then(Value::as_text)
        .ok_or_else(|| format!("github response missing {display_date}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/SubtitleEdit/subtitleedit/releases/latest"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "SubtitleEdit".to_string()),
            ("repo".to_string(), "subtitleedit".to_string()),
        ])
    }

    #[test]
    fn extracts_published_at_by_default() {
        let fetcher = FakeFetcher(
            r#"{"created_at": "2020-01-01T00:00:00Z", "published_at": "2020-01-02T00:00:00Z"}"#,
        );
        let value = resolve_release_date(&params(), &fetcher).unwrap();
        assert_eq!(value, "2020-01-02T00:00:00Z");
    }

    #[test]
    fn extracts_created_at_when_requested() {
        let fetcher = FakeFetcher(
            r#"{"created_at": "2020-01-01T00:00:00Z", "published_at": "2020-01-02T00:00:00Z"}"#,
        );
        let mut p = params();
        p.insert("display_date".to_string(), "created_at".to_string());
        let value = resolve_release_date(&p, &fetcher).unwrap();
        assert_eq!(value, "2020-01-01T00:00:00Z");
    }

    #[test]
    fn uses_the_releases_list_for_the_pre_variant() {
        struct PreFetcher;
        impl Fetcher for PreFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://api.github.com/repos/SubtitleEdit/subtitleedit/releases"
                );
                Ok(br#"[{"created_at": "2021-01-01T00:00:00Z", "published_at": "2021-01-02T00:00:00Z"}]"#.to_vec())
            }
        }
        let mut p = params();
        p.insert("variant".to_string(), "release-date-pre".to_string());
        let value = resolve_release_date(&p, &PreFetcher).unwrap();
        assert_eq!(value, "2021-01-02T00:00:00Z");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_release_date(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("repo".to_string(), String::new());
        assert!(resolve_release_date(&p, &Unused).is_err());
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
        assert!(resolve_release_date(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_display_date_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"created_at": "2020-01-01T00:00:00Z"}"#);
        assert!(resolve_release_date(&params(), &fetcher).is_err());
    }
}
