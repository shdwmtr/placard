use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let feed = params
        .get("feed")
        .ok_or("myget-downloads requires a data-feed attribute")?;
    let feed = validate_path_param("feed", feed)?;
    let package_name = params
        .get("package-name")
        .ok_or("myget-downloads requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;
    let subdomain = match params.get("tenant") {
        Some(v) if !v.is_empty() => validate_path_param("tenant", v)?,
        _ => "www",
    };

    let index_url = format!("https://{subdomain}.myget.org/F/{feed}/api/v3/index.json");
    let bytes = fetcher.fetch(&index_url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "myget response was not valid UTF-8".to_string())?;
    let index = json::parse(&text)?;
    let resources = match index.get("resources") {
        Some(Value::Array(items)) => items,
        _ => return Err("myget response missing resources array".to_string()),
    };
    let search_url = resources
        .iter()
        .find(|r| r.get("@type").and_then(|v| v.as_text()).as_deref() == Some("SearchQueryService"))
        .and_then(|r| r.get("@id"))
        .and_then(|v| v.as_text())
        .ok_or("myget response missing a SearchQueryService resource")?;

    let query_url = format!(
        "{search_url}?q=packageid%3A{}&prerelease=true&semVerLevel=2",
        package_name.to_lowercase()
    );
    let bytes = fetcher.fetch(&query_url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "myget response was not valid UTF-8".to_string())?;
    let search = json::parse(&text)?;
    let data = match search.get("data") {
        Some(Value::Array(items)) => items,
        _ => return Err("myget response missing data array".to_string()),
    };
    if data.len() != 1 {
        return Err("package not found".to_string());
    }
    let package = &data[0];
    match package
        .get("totalDownloads")
        .or_else(|| package.get("totaldownloads"))
    {
        Some(v) => v
            .as_text()
            .ok_or_else(|| "downloads count was not a plain value".to_string()),
        None => Ok("0".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeFetcher {
        index_response: &'static str,
        query_response: &'static str,
        expected_index_url: &'static str,
        expected_query_url: &'static str,
        calls: AtomicUsize,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => {
                    assert_eq!(url, self.expected_index_url);
                    Ok(self.index_response.as_bytes().to_vec())
                }
                1 => {
                    assert_eq!(url, self.expected_query_url);
                    Ok(self.query_response.as_bytes().to_vec())
                }
                _ => panic!("unexpected extra fetch"),
            }
        }
    }

    fn params(feed: &str, package_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("feed".to_string(), feed.to_string()),
            ("package-name".to_string(), package_name.to_string()),
        ])
    }

    const INDEX_BODY: &str = r#"{"resources": [{"@id": "https://www.myget.org/F/mongodb/api/v3/query", "@type": "SearchQueryService"}, {"@id": "other", "@type": "SearchAutocompleteService"}]}"#;

    #[test]
    fn extracts_total_downloads_using_the_default_www_tenant() {
        let fetcher = FakeFetcher {
            index_response: INDEX_BODY,
            query_response: r#"{"data": [{"totalDownloads": 12345}]}"#,
            expected_index_url: "https://www.myget.org/F/mongodb/api/v3/index.json",
            expected_query_url: "https://www.myget.org/F/mongodb/api/v3/query?q=packageid%3Amongodb.driver.core&prerelease=true&semVerLevel=2",
            calls: AtomicUsize::new(0),
        };
        let value = resolve_downloads(&params("mongodb", "MongoDB.Driver.Core"), &fetcher).unwrap();
        assert_eq!(value, "12345");
    }

    #[test]
    fn falls_back_to_lowercase_totaldownloads_field() {
        let fetcher = FakeFetcher {
            index_response: INDEX_BODY,
            query_response: r#"{"data": [{"totaldownloads": 99}]}"#,
            expected_index_url: "https://www.myget.org/F/mongodb/api/v3/index.json",
            expected_query_url: "https://www.myget.org/F/mongodb/api/v3/query?q=packageid%3Amongodb.driver.core&prerelease=true&semVerLevel=2",
            calls: AtomicUsize::new(0),
        };
        let value = resolve_downloads(&params("mongodb", "MongoDB.Driver.Core"), &fetcher).unwrap();
        assert_eq!(value, "99");
    }

    #[test]
    fn uses_a_custom_tenant_subdomain() {
        let mut p = params("vs-devcore", "MicroBuild");
        p.insert("tenant".to_string(), "vs-devcore".to_string());
        let fetcher = FakeFetcher {
            index_response: r#"{"resources": [{"@id": "https://vs-devcore.myget.org/F/vs-devcore/api/v3/query", "@type": "SearchQueryService"}]}"#,
            query_response: r#"{"data": [{"totalDownloads": 5}]}"#,
            expected_index_url: "https://vs-devcore.myget.org/F/vs-devcore/api/v3/index.json",
            expected_query_url: "https://vs-devcore.myget.org/F/vs-devcore/api/v3/query?q=packageid%3Amicrobuild&prerelease=true&semVerLevel=2",
            calls: AtomicUsize::new(0),
        };
        let value = resolve_downloads(&p, &fetcher).unwrap();
        assert_eq!(value, "5");
    }

    #[test]
    fn requires_feed_and_package_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        assert!(resolve_downloads(&params("mongodb", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc", "pkg"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_package_is_not_found() {
        let fetcher = FakeFetcher {
            index_response: INDEX_BODY,
            query_response: r#"{"data": []}"#,
            expected_index_url: "https://www.myget.org/F/mongodb/api/v3/index.json",
            expected_query_url: "https://www.myget.org/F/mongodb/api/v3/query?q=packageid%3Amongodb.driver.core&prerelease=true&semVerLevel=2",
            calls: AtomicUsize::new(0),
        };
        assert!(resolve_downloads(&params("mongodb", "MongoDB.Driver.Core"), &fetcher).is_err());
    }
}
