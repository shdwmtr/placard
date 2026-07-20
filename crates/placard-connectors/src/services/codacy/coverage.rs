use super::validate_path_param;
use crate::Fetcher;
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

/// Codacy coverage badge SVGs render their value in a
/// `text-anchor="middle">VALUE</text>` element.
fn extract_marked_text(svg: &str, marker: &str) -> Result<String, String> {
    let start = svg.find(marker).ok_or("unparseable svg response")?;
    let after = &svg[start + marker.len()..];
    let end = after.find(['<', '>']).ok_or("unparseable svg response")?;
    let candidate = &after[..end];
    if candidate.is_empty() {
        return Err("unparseable svg response".to_string());
    }
    Ok(candidate.to_string())
}

pub(crate) fn resolve_coverage(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_id = params
        .get("project-id")
        .ok_or("codacy-coverage requires a data-project-id attribute")?;
    let project_id = validate_path_param("project-id", project_id)?;

    let mut url = format!("https://api.codacy.com/project/badge/coverage/{project_id}");
    if let Some(branch) = params.get("branch") {
        if branch.is_empty() {
            return Err("'branch' parameter must not be empty".to_string());
        }
        url.push_str("?branch=");
        url.push_str(&percent_encode(branch));
    }

    let bytes = fetcher.fetch(&url)?;
    let svg =
        String::from_utf8(bytes).map_err(|_| "codacy response was not valid UTF-8".to_string())?;
    let percentage = extract_marked_text(&svg, "text-anchor=\"middle\">")?;

    if percentage == "!" {
        return Err("not enabled for this project".to_string());
    }
    if !percentage.ends_with('%')
        || !percentage[..percentage.len() - 1]
            .chars()
            .all(|c| c.is_ascii_digit())
    {
        return Err("unparseable coverage value".to_string());
    }
    Ok(percentage)
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

    fn params(project_id: &str) -> HashMap<String, String> {
        HashMap::from([("project-id".to_string(), project_id.to_string())])
    }

    fn badge_svg(value: &str) -> String {
        format!(r#"<svg><text x="50" text-anchor="middle">{value}</text></svg>"#)
    }

    #[test]
    fn extracts_the_coverage_percentage() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/coverage/84c0a068ce9349f2bcaa07b5977bd932",
            body: badge_svg("83%"),
        };
        let value =
            resolve_coverage(&params("84c0a068ce9349f2bcaa07b5977bd932"), &fetcher).unwrap();
        assert_eq!(value, "83%");
    }

    #[test]
    fn includes_the_branch_query_param_when_given() {
        let mut p = params("84c0a068ce9349f2bcaa07b5977bd932");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/coverage/84c0a068ce9349f2bcaa07b5977bd932?branch=master",
            body: badge_svg("42%"),
        };
        let value = resolve_coverage(&p, &fetcher).unwrap();
        assert_eq!(value, "42%");
    }

    #[test]
    fn requires_project_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project id")
            }
        }
        assert!(resolve_coverage(&HashMap::new(), &Unused).is_err());
        assert!(resolve_coverage(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project id")
            }
        }
        assert!(resolve_coverage(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_coverage_is_not_enabled() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/coverage/84c0a068ce9349f2bcaa07b5977bd932",
            body: badge_svg("!"),
        };
        assert!(resolve_coverage(&params("84c0a068ce9349f2bcaa07b5977bd932"), &fetcher).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/coverage/84c0a068ce9349f2bcaa07b5977bd932",
            body: "not an svg".to_string(),
        };
        assert!(resolve_coverage(&params("84c0a068ce9349f2bcaa07b5977bd932"), &fetcher).is_err());
    }
}
