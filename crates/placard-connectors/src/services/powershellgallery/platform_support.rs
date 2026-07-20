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

pub(crate) fn resolve_platform_support(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("powershellgallery-platform-support requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;
    let package_lower = package.to_ascii_lowercase();

    let mut xml = fetch_entry_xml(fetcher, &package_lower, false)?;
    let mut tags = extract_tag(&xml, "d:Tags");

    if tags.is_none() {
        xml = fetch_entry_xml(fetcher, &package_lower, true)?;
        tags = extract_tag(&xml, "d:Tags");
    }
    let tags = tags.ok_or_else(|| "powershellgallery response missing d:Tags".to_string())?;

    let mut platforms: Vec<&str> = Vec::new();
    for raw_tag in tags.split_whitespace() {
        let lower = raw_tag.to_ascii_lowercase();
        let matched = match lower.as_str() {
            "windows" => "windows",
            "macos" => "macos",
            "linux" => "linux",
            _ => continue,
        };
        if !platforms.contains(&matched) {
            platforms.push(matched);
        }
    }

    if platforms.is_empty() {
        return Ok("not specified".to_string());
    }
    Ok(platforms.join(" | "))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        stable_url: String,
        stable_body: Option<&'static str>,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.stable_url);
            Ok(self
                .stable_body
                .unwrap_or("<feed></feed>")
                .as_bytes()
                .to_vec())
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

    fn entry_xml(tags: &str) -> &'static str {
        Box::leak(
            format!(
                "<feed><entry><m:properties><d:Tags>{tags}</d:Tags></m:properties></entry></feed>"
            )
            .into_boxed_str(),
        )
    }

    fn stable_url() -> String {
        filter_url("packagemanagement", "IsLatestVersion eq true")
    }

    #[test]
    fn extracts_recognized_platforms_from_tags() {
        let fetcher = FakeFetcher {
            stable_url: stable_url(),
            stable_body: Some(entry_xml("Windows MacOS Linux CoreCLR")),
        };
        let value = resolve_platform_support(&params("PackageManagement"), &fetcher).unwrap();
        assert_eq!(value, "windows | macos | linux");
    }

    #[test]
    fn defaults_to_not_specified_when_no_platform_tags_match() {
        let fetcher = FakeFetcher {
            stable_url: stable_url(),
            stable_body: Some(entry_xml("PSModule PowerShell")),
        };
        let value = resolve_platform_support(&params("PackageManagement"), &fetcher).unwrap();
        assert_eq!(value, "not specified");
    }

    #[test]
    fn requires_the_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_platform_support(&HashMap::new(), &Unused).is_err());
        assert!(resolve_platform_support(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_platform_support(&params("../etc"), &Unused).is_err());
    }
}
