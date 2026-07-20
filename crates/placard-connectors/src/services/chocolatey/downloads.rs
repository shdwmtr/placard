use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

/// Extracts the text content of the first `<tag ...>text</tag>` element
/// (namespaced tag names like `d:DownloadCount` are matched verbatim),
/// tolerating attributes on the opening tag the way OData/Atom XML feeds
/// use them.
fn extract_element(xml: &str, tag: &str) -> Option<String> {
    let open_needle = format!("<{tag}");
    let start = xml.find(&open_needle)?;
    let rest = &xml[start + open_needle.len()..];
    let gt = rest.find('>')?;
    if rest.as_bytes().get(gt.wrapping_sub(1)).copied() == Some(b'/') {
        return Some(String::new());
    }
    let content_start = start + open_needle.len() + gt + 1;
    let close_needle = format!("</{tag}>");
    let close_rel = xml[content_start..].find(&close_needle)?;
    Some(
        xml[content_start..content_start + close_rel]
            .trim()
            .to_string(),
    )
}

/// Chocolatey is a NuGet v2 (OData/Atom XML) feed, not JSON. This queries
/// the `Packages()` OData resource for the package's latest entry and
/// scrapes the `d:DownloadCount` element out of the returned XML.
pub(crate) fn resolve_dt(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("chocolatey-dt requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let filter = format!(
        "tolower(Id) eq '{}' and IsLatestVersion eq true",
        package.to_lowercase()
    );
    let url = format!("https://community.chocolatey.org/api/v2/Packages()?$filter={filter}");

    let bytes = fetcher.fetch(&url)?;
    let xml = String::from_utf8(bytes)
        .map_err(|_| "chocolatey response was not valid UTF-8".to_string())?;
    if !xml.contains("<entry") {
        return Err("chocolatey response contains no matching package".to_string());
    }
    let downloads = extract_element(&xml, "d:DownloadCount")
        .ok_or("chocolatey response missing d:DownloadCount")?;
    if downloads.is_empty() || !downloads.chars().all(|c| c.is_ascii_digit()) {
        return Err("chocolatey response's DownloadCount element was not numeric".to_string());
    }
    Ok(downloads)
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

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    const ENTRY_XML: &str = r#"<feed><entry><m:properties>
        <d:Version>0.17.0</d:Version>
        <d:NormalizedVersion>0.17.0</d:NormalizedVersion>
        <d:DownloadCount m:type="Edm.Int32">123456</d:DownloadCount>
    </m:properties></entry></feed>"#;

    #[test]
    fn extracts_download_count_from_an_odata_feed() {
        let fetcher = FakeFetcher {
            expected_url: "https://community.chocolatey.org/api/v2/Packages()?$filter=tolower(Id) eq 'git' and IsLatestVersion eq true",
            body: ENTRY_XML,
        };
        let value = resolve_dt(&params("git"), &fetcher).unwrap();
        assert_eq!(value, "123456");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_dt(&HashMap::new(), &Unused).is_err());
        assert!(resolve_dt(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_dt(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_entry_is_returned() {
        let fetcher = FakeFetcher {
            expected_url: "https://community.chocolatey.org/api/v2/Packages()?$filter=tolower(Id) eq 'not-a-real-package' and IsLatestVersion eq true",
            body: r#"<feed></feed>"#,
        };
        assert!(resolve_dt(&params("not-a-real-package"), &fetcher).is_err());
    }
}
