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

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package")
        .ok_or("nuget-downloads requires a data-package attribute")?;
    let package_name = validate_path_param("package", package_name)?;

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

    let downloads = pkg_fields
        .iter()
        .find(|(k, _)| k == "totalDownloads")
        .or_else(|| pkg_fields.iter().find(|(k, _)| k == "totaldownloads"))
        .and_then(|(_, v)| v.as_text())
        .unwrap_or_else(|| "0".to_string());
    Ok(downloads)
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
    fn extracts_total_downloads() {
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:microsoft.aspnet.mvc&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": [{"totalDownloads": 123456789, "versions": []}]}"#,
        };
        let value = resolve_downloads(&params("Microsoft.AspNet.Mvc"), &fetcher).unwrap();
        assert_eq!(value, "123456789");
    }

    #[test]
    fn falls_back_to_lowercase_totaldownloads_key() {
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:some.myget.package&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": [{"totaldownloads": 42, "versions": []}]}"#,
        };
        let value = resolve_downloads(&params("Some.MyGet.Package"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn errors_when_package_not_found() {
        let fetcher = FakeFetcher {
            index_body: INDEX_BODY,
            expected_search_url: "https://azuresearch-usnc.nuget.org/query?q=packageid:nonexistent&prerelease=true&semVerLevel=2",
            search_body: r#"{"data": []}"#,
        };
        assert!(resolve_downloads(&params("NonExistent"), &fetcher).is_err());
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package param")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid package param")
            }
        }
        assert!(resolve_downloads(&params("../etc/passwd"), &Unused).is_err());
    }
}
