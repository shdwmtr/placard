use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

/// Finds the text content of the first `<tag ...>text</tag>` element,
/// tolerating attributes on the opening tag (as NuGet v2's OData/Atom
/// feed puts on its `d:*` properties, e.g. `m:type="Edm.String"`).
fn extract_tag_text(xml: &str, tag: &str) -> Option<String> {
    let open_marker = format!("<{tag}");
    let open_start = xml.find(&open_marker)?;
    let after_open = &xml[open_start + open_marker.len()..];
    let tag_end = after_open.find('>')?;
    if after_open[..tag_end].ends_with('/') {
        return None;
    }
    let content_start = tag_end + 1;
    let close_marker = format!("</{tag}>");
    let close_start = after_open[content_start..].find(&close_marker)?;
    let text = after_open[content_start..content_start + close_start].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

pub(crate) fn resolve_resharper(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("resharper requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;
    let lowered = package_name.to_ascii_lowercase();

    let url = format!(
        "https://resharper-plugins.jetbrains.com/api/v2/Packages()?$filter=tolower%28Id%29%20eq%20%27{lowered}%27%20and%20IsLatestVersion%20eq%20true"
    );
    let bytes = fetcher.fetch(&url)?;
    let xml = String::from_utf8(bytes)
        .map_err(|_| "resharper response was not valid UTF-8".to_string())?;

    extract_tag_text(&xml, "d:NormalizedVersion")
        .or_else(|| extract_tag_text(&xml, "d:Version"))
        .ok_or("resharper response missing version".to_string())
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

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    const EXPECTED_URL: &str = "https://resharper-plugins.jetbrains.com/api/v2/Packages()?$filter=tolower%28Id%29%20eq%20%27stylecop.stylecop%27%20and%20IsLatestVersion%20eq%20true";

    #[test]
    fn extracts_the_normalized_version_from_the_odata_feed() {
        let fetcher = FakeFetcher {
            expected_url: EXPECTED_URL,
            body: r#"<feed><entry><m:properties>
                <d:Version m:type="Edm.String">1.2.0.0</d:Version>
                <d:NormalizedVersion m:type="Edm.String">1.2.0</d:NormalizedVersion>
                <d:DownloadCount m:type="Edm.Int32">42</d:DownloadCount>
            </m:properties></entry></feed>"#,
        };
        let value = resolve_resharper(&params("StyleCop.StyleCop"), &fetcher).unwrap();
        assert_eq!(value, "1.2.0");
    }

    #[test]
    fn falls_back_to_version_when_normalized_version_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: EXPECTED_URL,
            body: r#"<feed><entry><m:properties>
                <d:Version m:type="Edm.String">1.2.0.0</d:Version>
            </m:properties></entry></feed>"#,
        };
        let value = resolve_resharper(&params("StyleCop.StyleCop"), &fetcher).unwrap();
        assert_eq!(value, "1.2.0.0");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_resharper(&HashMap::new(), &Unused).is_err());
        assert!(resolve_resharper(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_resharper(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_version_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: EXPECTED_URL,
            body: r#"<feed></feed>"#,
        };
        assert!(resolve_resharper(&params("StyleCop.StyleCop"), &fetcher).is_err());
    }
}
