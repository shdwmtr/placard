use crate::Fetcher;
use crate::services::validate_path_param;
use std::collections::HashMap;

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open_prefix = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = xml.find(&open_prefix)?;
    let after_open = &xml[start + open_prefix.len()..];
    let gt = after_open.find('>')?;
    let content_start = start + open_prefix.len() + gt + 1;
    let end = xml[content_start..].find(&close)? + content_start;
    let text = xml[content_start..end].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn fetch_entry_xml(
    fetcher: &dyn Fetcher,
    package_lower: &str,
    include_prereleases: bool,
) -> Result<String, String> {
    let release_filter = if include_prereleases {
        "IsAbsoluteLatestVersion eq true"
    } else {
        "IsLatestVersion eq true"
    };
    let filter = format!("tolower(Id) eq '{package_lower}' and {release_filter}");
    let url = format!(
        "https://www.powershellgallery.com/api/v2/Packages()?$filter={}",
        percent_encode(&filter)
    );
    let bytes = fetcher.fetch(&url)?;
    String::from_utf8(bytes)
        .map_err(|_| "powershellgallery response was not valid UTF-8".to_string())
}

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("powershellgallery-downloads requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;
    let package_lower = package.to_ascii_lowercase();

    let mut xml = fetch_entry_xml(fetcher, &package_lower, false)?;
    let mut downloads = extract_tag(&xml, "d:DownloadCount");

    if downloads.is_none() {
        xml = fetch_entry_xml(fetcher, &package_lower, true)?;
        downloads = extract_tag(&xml, "d:DownloadCount");
    }

    downloads.ok_or_else(|| "powershellgallery response missing d:DownloadCount".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        stable_url: String,
        stable_body: Option<&'static str>,
        prerelease_url: Option<String>,
        prerelease_body: Option<&'static str>,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            if url == self.stable_url {
                return Ok(self
                    .stable_body
                    .unwrap_or("<feed></feed>")
                    .as_bytes()
                    .to_vec());
            }
            if Some(url.to_string()) == self.prerelease_url {
                return Ok(self
                    .prerelease_body
                    .unwrap_or("<feed></feed>")
                    .as_bytes()
                    .to_vec());
            }
            panic!("unexpected url: {url}");
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    fn filter_url(package_lower: &str, release_filter: &str) -> String {
        let filter = format!("tolower(Id) eq '{package_lower}' and {release_filter}");
        format!(
            "https://www.powershellgallery.com/api/v2/Packages()?$filter={}",
            percent_encode(&filter)
        )
    }

    fn entry_xml(downloads: u64) -> &'static str {
        Box::leak(
            format!("<feed><entry><m:properties><d:DownloadCount m:type=\"Edm.Int32\">{downloads}</d:DownloadCount></m:properties></entry></feed>")
                .into_boxed_str(),
        )
    }

    #[test]
    fn extracts_the_download_count() {
        let fetcher = FakeFetcher {
            stable_url: filter_url("azure.storage", "IsLatestVersion eq true"),
            stable_body: Some(entry_xml(48213)),
            prerelease_url: None,
            prerelease_body: None,
        };
        let value = resolve_downloads(&params("Azure.Storage"), &fetcher).unwrap();
        assert_eq!(value, "48213");
    }

    #[test]
    fn falls_back_to_prereleases_when_no_stable_version_exists() {
        let fetcher = FakeFetcher {
            stable_url: filter_url("azure.storage", "IsLatestVersion eq true"),
            stable_body: None,
            prerelease_url: Some(filter_url(
                "azure.storage",
                "IsAbsoluteLatestVersion eq true",
            )),
            prerelease_body: Some(entry_xml(10)),
        };
        let value = resolve_downloads(&params("Azure.Storage"), &fetcher).unwrap();
        assert_eq!(value, "10");
    }

    #[test]
    fn requires_the_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
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
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_downloads(&params("../etc"), &Unused).is_err());
    }
}
