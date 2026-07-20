use super::super::validate_path_param;
use crate::json::Value;
use crate::{Fetcher, json};
use std::collections::HashMap;

fn strip_build_metadata(version: &str) -> &str {
    version.split('+').next().unwrap_or(version)
}

fn is_prerelease(version: &str) -> bool {
    version.contains('-')
}

/// Mirrors upstream's `selectVersion`: picks the last (highest) entry from
/// the feed's `versions` array, optionally restricted to non-prerelease
/// versions. The feed already returns versions in ascending order, so this
/// is a plain "last matching entry" pick rather than a semver comparison.
fn select_version(versions: &[String], include_prereleases: bool) -> Option<String> {
    if include_prereleases {
        return versions.last().cloned();
    }
    versions
        .iter()
        .filter(|v| !is_prerelease(v))
        .last()
        .or_else(|| versions.last())
        .cloned()
}

pub(crate) fn resolve_feedz(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let organization = params
        .get("organization")
        .ok_or("feedz requires a data-organization attribute")?;
    let organization = validate_path_param("organization", organization)?;
    let repository = params
        .get("repository")
        .ok_or("feedz requires a data-repository attribute")?;
    let repository = validate_path_param("repository", repository)?;
    let package_name = params
        .get("package-name")
        .ok_or("feedz requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;
    let variant = params.get("variant").map(String::as_str).unwrap_or("v");
    if variant != "v" && variant != "vpre" {
        return Err("'variant' parameter must be 'v' or 'vpre'".to_string());
    }
    let include_prereleases = variant == "vpre";

    let url = format!(
        "https://f.feedz.io/{organization}/{repository}/nuget/v3/packages/{package_name}/index.json"
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "feedz response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let Some(Value::Array(items)) = value.get("versions") else {
        return Err("feedz response missing versions array".to_string());
    };
    let versions: Vec<String> = items
        .iter()
        .filter_map(|v| v.as_text())
        .map(|v| strip_build_metadata(&v).to_string())
        .collect();
    if versions.is_empty() {
        return Err("repository or package not found".to_string());
    }

    select_version(&versions, include_prereleases)
        .ok_or_else(|| "no matching version found".to_string())
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

    fn params(organization: &str, repository: &str, package_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("organization".to_string(), organization.to_string()),
            ("repository".to_string(), repository.to_string()),
            ("package-name".to_string(), package_name.to_string()),
        ])
    }

    #[test]
    fn extracts_the_latest_stable_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://f.feedz.io/shieldstests/mongodb/nuget/v3/packages/MongoDB.Driver.Core/index.json",
            body: r#"{"versions": ["1.0.0", "1.1.0-beta1", "1.1.0"]}"#,
        };
        let value = resolve_feedz(
            &params("shieldstests", "mongodb", "MongoDB.Driver.Core"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "1.1.0");
    }

    #[test]
    fn includes_prereleases_when_variant_is_vpre() {
        let fetcher = FakeFetcher {
            expected_url: "https://f.feedz.io/shieldstests/mongodb/nuget/v3/packages/MongoDB.Driver.Core/index.json",
            body: r#"{"versions": ["1.0.0", "1.1.0-beta1"]}"#,
        };
        let mut p = params("shieldstests", "mongodb", "MongoDB.Driver.Core");
        p.insert("variant".to_string(), "vpre".to_string());
        let value = resolve_feedz(&p, &fetcher).unwrap();
        assert_eq!(value, "1.1.0-beta1");
    }

    #[test]
    fn strips_build_metadata() {
        let fetcher = FakeFetcher {
            expected_url: "https://f.feedz.io/shieldstests/mongodb/nuget/v3/packages/MongoDB.Driver.Core/index.json",
            body: r#"{"versions": ["1.0.0+build123"]}"#,
        };
        let value = resolve_feedz(
            &params("shieldstests", "mongodb", "MongoDB.Driver.Core"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "1.0.0");
    }

    #[test]
    fn requires_organization_repository_and_package_name() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_feedz(&HashMap::new(), &Unused).is_err());
        assert!(resolve_feedz(&params("", "mongodb", "MongoDB.Driver.Core"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_feedz(&params("../etc", "mongodb", "x"), &Unused).is_err());
    }

    #[test]
    fn errors_when_versions_are_empty() {
        let fetcher = FakeFetcher {
            expected_url: "https://f.feedz.io/shieldstests/mongodb/nuget/v3/packages/MongoDB.Driver.Core/index.json",
            body: r#"{"versions": []}"#,
        };
        assert!(
            resolve_feedz(
                &params("shieldstests", "mongodb", "MongoDB.Driver.Core"),
                &fetcher
            )
            .is_err()
        );
    }
}
