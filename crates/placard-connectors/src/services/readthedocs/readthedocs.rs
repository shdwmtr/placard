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

/// Read the Docs badge SVGs render their value as the last
/// `<text>...</text></g>` element, the same shape shields' generic SVG
/// badge scraper reads.
fn extract_svg_text(svg: &str) -> Result<String, String> {
    let stripped = strip_svg_whitespace(svg);
    let marker = "</text></g>";
    let end = stripped.rfind(marker).ok_or("unparseable svg response")?;
    let before = &stripped[..end];
    let start = before.rfind('>').ok_or("unparseable svg response")?;
    let candidate = &before[start + 1..];
    if candidate.is_empty() || candidate.contains('<') {
        return Err("unparseable svg response".to_string());
    }
    Ok(candidate.to_string())
}

fn strip_svg_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' || c == '\r' {
            while matches!(chars.peek(), Some(next) if next.is_whitespace()) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub(crate) fn resolve_readthedocs(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("readthedocs requires a data-project attribute")?;
    let project = validate_path_param("project", project)?;

    let mut url = format!("https://readthedocs.org/projects/{project}/badge/");
    if let Some(version) = params.get("version") {
        if version.is_empty() {
            return Err("'version' parameter must not be empty".to_string());
        }
        url.push_str("?version=");
        url.push_str(&percent_encode(version));
    }

    let bytes = fetcher.fetch(&url)?;
    let svg = String::from_utf8(bytes)
        .map_err(|_| "readthedocs response was not valid UTF-8".to_string())?;
    let status = extract_svg_text(&svg)?;

    if status == "unknown" {
        return Err("project or version not found".to_string());
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: String,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(project: &str) -> HashMap<String, String> {
        HashMap::from([("project".to_string(), project.to_string())])
    }

    fn badge_svg(value: &str) -> String {
        format!("<svg><g><text>docs</text></g><g><text>{value}</text></g></svg>")
    }

    #[test]
    fn extracts_the_build_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://readthedocs.org/projects/pip/badge/",
            body: badge_svg("passing"),
        };
        let value = resolve_readthedocs(&params("pip"), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn includes_the_version_query_param_when_given() {
        let mut p = params("pip");
        p.insert("version".to_string(), "stable".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://readthedocs.org/projects/pip/badge/?version=stable",
            body: badge_svg("passing"),
        };
        let value = resolve_readthedocs(&p, &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn errors_when_the_project_or_version_is_unknown() {
        let fetcher = FakeFetcher {
            expected_url: "https://readthedocs.org/projects/pip/badge/",
            body: badge_svg("unknown"),
        };
        assert!(resolve_readthedocs(&params("pip"), &fetcher).is_err());
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_readthedocs(&HashMap::new(), &Unused).is_err());
        assert!(resolve_readthedocs(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_readthedocs(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://readthedocs.org/projects/pip/badge/",
            body: "not an svg".to_string(),
        };
        assert!(resolve_readthedocs(&params("pip"), &fetcher).is_err());
    }
}
