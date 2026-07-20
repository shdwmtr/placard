use crate::Fetcher;
use std::collections::HashMap;

fn validate_metadata_url(url: &str) -> Result<&str, String> {
    if url.is_empty() {
        return Err("'metadata_url' parameter must not be empty".to_string());
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(
            "'metadata_url' parameter must be a well-formed http:// or https:// URL".to_string(),
        );
    }
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(
            "'metadata_url' parameter contains disallowed whitespace or control characters"
                .to_string(),
        );
    }
    Ok(url)
}

fn extract_element(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let text = xml[start..end].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// Upstream shields picks the "highest" version by sorting every listed
/// version with a Maven-specific comparator (its default `highestVersion`
/// strategy). Reproducing that ordering isn't plain field extraction, so
/// this instead reports the `release` version the repository itself
/// publishes in the metadata document (falling back to `latest`), which is
/// what shields' own `releaseProperty`/`latestProperty` strategies use.
pub(crate) fn resolve_maven_metadata(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let metadata_url = params
        .get("metadata_url")
        .ok_or("maven-metadata requires a data-metadata_url attribute")?;
    let metadata_url = validate_metadata_url(metadata_url)?;

    let bytes = fetcher.fetch(metadata_url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "maven-metadata response was not valid UTF-8".to_string())?;

    extract_element(&text, "release")
        .or_else(|| extract_element(&text, "latest"))
        .ok_or_else(|| "maven-metadata response missing release/latest version".to_string())
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

    fn params(metadata_url: &str) -> HashMap<String, String> {
        HashMap::from([("metadata_url".to_string(), metadata_url.to_string())])
    }

    #[test]
    fn extracts_the_release_field() {
        let fetcher = FakeFetcher {
            expected_url: "https://repo1.maven.org/maven2/com/google/guava/guava/maven-metadata.xml",
            body: r#"<metadata><versioning><latest>33.0.0</latest><release>33.0.0</release>
                <versions><version>32.0.0</version><version>33.0.0</version></versions>
                </versioning></metadata>"#,
        };
        let value = resolve_maven_metadata(
            &params("https://repo1.maven.org/maven2/com/google/guava/guava/maven-metadata.xml"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "33.0.0");
    }

    #[test]
    fn falls_back_to_latest_when_release_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://example.com/maven-metadata.xml",
            body: r#"<metadata><versioning><latest>1.2.3-SNAPSHOT</latest>
                <versions><version>1.2.3-SNAPSHOT</version></versions></versioning></metadata>"#,
        };
        let value =
            resolve_maven_metadata(&params("https://example.com/maven-metadata.xml"), &fetcher)
                .unwrap();
        assert_eq!(value, "1.2.3-SNAPSHOT");
    }

    #[test]
    fn requires_a_metadata_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid metadata_url")
            }
        }
        assert!(resolve_maven_metadata(&HashMap::new(), &Unused).is_err());
        assert!(resolve_maven_metadata(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        assert!(resolve_maven_metadata(&params("file:///etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_neither_release_nor_latest_is_present() {
        let fetcher = FakeFetcher {
            expected_url: "https://example.com/maven-metadata.xml",
            body: r#"<metadata><versioning><versions><version>1.0</version></versions></versioning></metadata>"#,
        };
        assert!(
            resolve_maven_metadata(&params("https://example.com/maven-metadata.xml"), &fetcher)
                .is_err()
        );
    }
}
