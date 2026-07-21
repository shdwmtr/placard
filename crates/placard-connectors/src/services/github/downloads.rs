use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn fetch_json(fetcher: &dyn Fetcher, url: &str) -> Result<Value, String> {
    let bytes = fetcher.fetch(url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    json::parse(&text)
}

fn fetch_release(fetcher: &dyn Fetcher, url: &str) -> Result<Value, String> {
    let value = fetch_json(fetcher, url)?;
    match value {
        Value::Object(_) => Ok(value),
        _ => Err("github response was not a JSON object".to_string()),
    }
}

fn fetch_releases(fetcher: &dyn Fetcher, url: &str) -> Result<Vec<Value>, String> {
    let value = fetch_json(fetcher, url)?;
    match value {
        Value::Array(items) => Ok(items),
        _ => Err("github response was not a JSON array".to_string()),
    }
}

fn extract_assets(release: &Value) -> Result<Vec<Value>, String> {
    match release.get("assets") {
        Some(Value::Array(items)) => Ok(items.clone()),
        _ => Err("github response missing assets array".to_string()),
    }
}

fn is_prerelease(release: &Value) -> bool {
    matches!(release.get("prerelease"), Some(Value::Bool(true)))
}

fn fetch_latest_release(
    fetcher: &dyn Fetcher,
    owner: &str,
    repo: &str,
    include_prereleases: bool,
) -> Result<Value, String> {
    if !include_prereleases {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
        return fetch_release(fetcher, &url);
    }

    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=100");
    let releases = fetch_releases(fetcher, &url)?;
    releases
        .into_iter()
        .next()
        .ok_or_else(|| "no releases found".to_string())
}

fn asset_download_count(asset: &Value) -> Result<f64, String> {
    match asset.get("download_count") {
        Some(Value::Number(n)) => Ok(*n),
        _ => Err("asset missing download_count".to_string()),
    }
}

fn sum_matching_assets(assets: &[Value], asset_name: Option<&str>) -> Result<f64, String> {
    let mut total = 0f64;
    for asset in assets {
        let matches = match asset_name {
            None | Some("total") => true,
            Some(wanted) => asset
                .get("name")
                .and_then(Value::as_text)
                .is_some_and(|name| name.eq_ignore_ascii_case(wanted)),
        };
        if matches {
            total += asset_download_count(asset)?;
        }
    }
    Ok(total)
}

fn format_total(total: f64) -> String {
    if total.fract() == 0.0 && total.abs() < 1e15 {
        format!("{}", total as i64)
    } else {
        total.to_string()
    }
}

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

    let include_prereleases = match params.get("variant").map(String::as_str) {
        None | Some("downloads") => false,
        Some("downloads-pre") => true,
        Some(other) => {
            return Err(format!(
                "github-downloads: 'variant' parameter '{other}' is not one of downloads, downloads-pre"
            ));
        }
    };

    match params.get("sort").map(String::as_str) {
        None | Some("date") => {}
        Some("semver") => {
            return Err("github-downloads: data-sort=\"semver\" is not supported".to_string());
        }
        Some(other) => {
            return Err(format!(
                "github-downloads: unsupported data-sort value '{other}'"
            ));
        }
    }

    let asset_name = params.get("asset_name").map(String::as_str);

    let assets = match params.get("tag").map(String::as_str) {
        Some(tag) if tag != "latest" => {
            let tag = validate_path_param("tag", tag)?;
            let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}");
            extract_assets(&fetch_release(fetcher, &url)?)?
        }
        Some(_latest) => {
            let release = fetch_latest_release(fetcher, owner, repo, include_prereleases)?;
            extract_assets(&release)?
        }
        None => {
            let url = format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=100");
            let releases = fetch_releases(fetcher, &url)?;
            if releases.is_empty() {
                return Err("github-downloads: no releases found".to_string());
            }
            let wanted_releases: Vec<&Value> = if include_prereleases {
                releases.iter().collect()
            } else {
                releases.iter().filter(|r| !is_prerelease(r)).collect()
            };
            let mut assets = Vec::new();
            for release in wanted_releases {
                assets.extend(extract_assets(release)?);
            }
            assets
        }
    };

    let total = sum_matching_assets(&assets, asset_name)?;
    Ok(format_total(total))
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
    fn sums_download_counts_across_assets_and_releases_by_default() {
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases?per_page=100",
            r#"[
                {"assets": [{"name": "a.deb", "download_count": 100}], "prerelease": false},
                {"assets": [{"name": "b.rpm", "download_count": 42}], "prerelease": false}
            ]"#,
        );
        let value = resolve_downloads(&params("atom", "atom"), &fetcher).unwrap();
        assert_eq!(value, "142");
    }

    #[test]
    fn excludes_prereleases_from_the_all_releases_total_by_default() {
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases?per_page=100",
            r#"[
                {"assets": [{"name": "a.deb", "download_count": 100}], "prerelease": true},
                {"assets": [{"name": "b.rpm", "download_count": 42}], "prerelease": false}
            ]"#,
        );
        let value = resolve_downloads(&params("atom", "atom"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn includes_prereleases_in_the_all_releases_total_for_the_pre_variant() {
        let mut p = params("atom", "atom");
        p.insert("variant".to_string(), "downloads-pre".to_string());
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases?per_page=100",
            r#"[
                {"assets": [{"name": "a.deb", "download_count": 100}], "prerelease": true},
                {"assets": [{"name": "b.rpm", "download_count": 42}], "prerelease": false}
            ]"#,
        );
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "142");
    }

    #[test]
    fn uses_the_releases_latest_endpoint_for_the_default_latest_tag() {
        let mut p = params("atom", "atom");
        p.insert("tag".to_string(), "latest".to_string());
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases/latest",
            r#"{"assets": [{"name": "a.deb", "download_count": 100}, {"name": "b.rpm", "download_count": 42}]}"#,
        );
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "142");
    }

    #[test]
    fn uses_the_releases_list_for_latest_with_the_pre_variant() {
        let mut p = params("atom", "atom");
        p.insert("tag".to_string(), "latest".to_string());
        p.insert("variant".to_string(), "downloads-pre".to_string());
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases?per_page=100",
            r#"[{"assets": [{"name": "a.deb", "download_count": 7}], "prerelease": true}]"#,
        );
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "7");
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
    fn filters_to_a_single_asset_by_name_case_insensitively() {
        let mut p = params("atom", "atom");
        p.insert("tag".to_string(), "v1.0.0".to_string());
        p.insert("asset_name".to_string(), "ATOM-AMD64.DEB".to_string());
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases/tags/v1.0.0",
            r#"{"assets": [
                {"name": "atom-amd64.deb", "download_count": 100},
                {"name": "atom.rpm", "download_count": 42}
            ]}"#,
        );
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "100");
    }

    #[test]
    fn treats_total_as_an_explicit_all_assets_keyword() {
        let mut p = params("atom", "atom");
        p.insert("tag".to_string(), "v1.0.0".to_string());
        p.insert("asset_name".to_string(), "total".to_string());
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases/tags/v1.0.0",
            r#"{"assets": [{"name": "a.deb", "download_count": 100}, {"name": "b.rpm", "download_count": 42}]}"#,
        );
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "142");
    }

    #[test]
    fn rejects_unsupported_variants() {
        let mut p = params("atom", "atom");
        p.insert("variant".to_string(), "downloads-nightly".to_string());
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid variant")
            }
        }
        assert!(resolve_downloads(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_unsupported_semver_sort() {
        let mut p = params("atom", "atom");
        p.insert("sort".to_string(), "semver".to_string());
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an unsupported sort")
            }
        }
        let err = resolve_downloads(&p, &Unused).unwrap_err();
        assert!(err.contains("not supported"));
    }

    #[test]
    fn errors_when_no_releases_exist() {
        let fetcher = FakeFetcher(
            "https://api.github.com/repos/atom/atom/releases?per_page=100",
            "[]",
        );
        assert!(resolve_downloads(&params("atom", "atom"), &fetcher).is_err());
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
            "https://api.github.com/repos/atom/atom/releases?per_page=100",
            r#"[{"id": 1, "prerelease": false}]"#,
        );
        assert!(resolve_downloads(&params("atom", "atom"), &fetcher).is_err());
    }
}
