use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

/// NuGet's v3 API is service-discovery based: the fixed `index.json`
/// resource lists a set of typed sub-services, one of which
/// (`SearchQueryService`) is the actual package search endpoint to query.
fn discover_search_service_url(fetcher: &dyn Fetcher) -> Result<String, String> {
    let bytes = fetcher.fetch("https://api.nuget.org/v3/index.json")?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "nuget index response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;

    let Value::Object(fields) = &doc else {
        return Err("nuget index response was not an object".to_string());
    };
    let resources = fields
        .iter()
        .find(|(k, _)| k == "resources")
        .map(|(_, v)| v)
        .ok_or("nuget index response missing resources")?;
    let Value::Array(items) = resources else {
        return Err("nuget index response's resources field was not an array".to_string());
    };
    for item in items {
        let Value::Object(item_fields) = item else {
            continue;
        };
        let kind = item_fields
            .iter()
            .find(|(k, _)| k == "@type")
            .and_then(|(_, v)| v.as_text());
        if kind.as_deref() == Some("SearchQueryService") {
            let id = item_fields
                .iter()
                .find(|(k, _)| k == "@id")
                .and_then(|(_, v)| v.as_text());
            return id.ok_or_else(|| "nuget SearchQueryService entry missing @id".to_string());
        }
    }
    Err("nuget index response has no SearchQueryService resource".to_string())
}

/// NuGet versions may carry an optional `+buildmetadata` suffix.
fn strip_build_metadata(version: &str) -> &str {
    version.split('+').next().unwrap_or(version)
}

/// Mirrors shields' `selectVersion`: with prereleases included, the last
/// (highest, since the search API returns versions ascending) entry wins
/// outright; otherwise the last entry lacking a `-prerelease` suffix wins,
/// falling back to the raw last entry if every version is a prerelease.
fn select_version(versions: &[String], include_prereleases: bool) -> Option<String> {
    if include_prereleases {
        return versions.last().cloned();
    }
    if let Some(last_stable) = versions.iter().rev().find(|v| !v.contains('-')) {
        return Some(last_stable.clone());
    }
    versions.last().cloned()
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package")
        .ok_or("nuget-version requires a data-package attribute")?;
    let package_name = validate_path_param("package", package_name)?;
    let include_prereleases = match params.get("variant").map(String::as_str) {
        None => false,
        Some("v") => false,
        Some("vpre") => true,
        Some(other) => return Err(format!("unknown nuget-version variant '{other}'")),
    };

    let search_url = discover_search_service_url(fetcher)?;
    let query_id = package_name.to_lowercase();
    let url = format!("{search_url}?q=packageid:{query_id}&prerelease=true&semVerLevel=2");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "nuget search response was not valid UTF-8".to_string())?;
    let doc = json::parse(&text)?;

    let Value::Object(fields) = &doc else {
        return Err("nuget search response was not an object".to_string());
    };
    let data = fields
        .iter()
        .find(|(k, _)| k == "data")
        .map(|(_, v)| v)
        .ok_or("nuget response missing data")?;
    let Value::Array(items) = data else {
        return Err("nuget response's data field was not an array".to_string());
    };
    let first = items.first().ok_or("package not found")?;
    let Value::Object(pkg_fields) = first else {
        return Err("nuget package entry was not an object".to_string());
    };
    let versions_field = pkg_fields
        .iter()
        .find(|(k, _)| k == "versions")
        .map(|(_, v)| v)
        .ok_or("nuget package entry missing versions")?;
    let Value::Array(version_items) = versions_field else {
        return Err("nuget package's versions field was not an array".to_string());
    };

    let versions: Vec<String> = version_items
        .iter()
        .filter_map(|item| {
            let Value::Object(vf) = item else { return None };
            vf.iter()
                .find(|(k, _)| k == "version")
                .and_then(|(_, v)| v.as_text())
        })
        .map(|v| strip_build_metadata(&v).to_string())
        .collect();

    select_version(&versions, include_prereleases).ok_or_else(|| "package not found".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        index_body: &'static str,
        expected_search_url: &'static str,
        search_body: &'static str,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            if url == "https://api.nuget.org/v3/index.json" {
                Ok(self.index_body.as_bytes().to_vec())
            } else {
                assert_eq!(url, self.expected_search_url);
                Ok(self.search_body.as_bytes().to_vec())
            }
        }
    }

    const INDEX_BODY: &str = r#"{"resources": [
        {"@id": "https://azuresearch-usnc.nuget.org/query", "@type": "SearchQueryService"},
        {"@id": "https://api.nuget.org/v3/registration5-semver1/", "@type": "RegistrationsBaseUrl"}
    ]}"#;

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn selects_the_latest_stable_version() {
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:microsoft.aspnet.mvc&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": [{"versions": [
                {"version": "5.2.6"},
                {"version": "5.2.7-beta1"},
                {"version": "5.2.7"}
            ]}]}"#,
        };
        let value = resolve_version(&params("Microsoft.AspNet.Mvc"), &fetcher).unwrap();
        assert_eq!(value, "5.2.7");
    }

    #[test]
    fn includes_prereleases_when_variant_is_vpre() {
        let mut p = params("Microsoft.AspNet.Mvc");
        p.insert("variant".to_string(), "vpre".to_string());
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:microsoft.aspnet.mvc&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": [{"versions": [
                {"version": "5.2.6"},
                {"version": "5.2.7-beta1"}
            ]}]}"#,
        };
        let value = resolve_version(&p, &fetcher).unwrap();
        assert_eq!(value, "5.2.7-beta1");
    }

    #[test]
    fn strips_build_metadata() {
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:mypkg&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": [{"versions": [{"version": "1.0.0+build.5"}]}]}"#,
        };
        let value = resolve_version(&params("mypkg"), &fetcher).unwrap();
        assert_eq!(value, "1.0.0");
    }

    #[test]
    fn falls_back_to_the_last_version_when_all_are_prereleases() {
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:mypkg&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": [{"versions": [{"version": "1.0.0-alpha"}, {"version": "1.0.0-beta"}]}]}"#,
        };
        let value = resolve_version(&params("mypkg"), &fetcher).unwrap();
        assert_eq!(value, "1.0.0-beta");
    }

    #[test]
    fn errors_when_package_not_found() {
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:nonexistent&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": []}"#,
        };
        assert!(resolve_version(&params("NonExistent"), &fetcher).is_err());
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid package param")
            }
        }
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn rejects_unknown_variants() {
        let mut p = params("mypkg");
        p.insert("variant".to_string(), "bogus".to_string());
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an unknown variant")
            }
        }
        assert!(resolve_version(&p, &Unused).is_err());
    }
}
