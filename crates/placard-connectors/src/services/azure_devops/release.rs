use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

/// Azure DevOps release badge SVGs render their value as the last
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

pub(crate) fn resolve_release(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let organization = params
        .get("organization")
        .ok_or("azure-devops-release requires a data-organization attribute")?;
    let project_id = params
        .get("project-id")
        .ok_or("azure-devops-release requires a data-project-id attribute")?;
    let definition_id = params
        .get("definition-id")
        .ok_or("azure-devops-release requires a data-definition-id attribute")?;
    let environment_id = params
        .get("environment-id")
        .ok_or("azure-devops-release requires a data-environment-id attribute")?;
    let organization = validate_path_param("organization", organization)?;
    let project_id = validate_path_param("project-id", project_id)?;
    let definition_id = validate_path_param("definition-id", definition_id)?;
    let environment_id = validate_path_param("environment-id", environment_id)?;

    let url = format!(
        "https://vsrm.dev.azure.com/{organization}/_apis/public/Release/badge/{project_id}/{definition_id}/{environment_id}"
    );
    let bytes = fetcher.fetch(&url)?;
    let svg = String::from_utf8(bytes)
        .map_err(|_| "azure devops response was not valid UTF-8".to_string())?;
    extract_svg_text(&svg)
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

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("organization".to_string(), "totodem".to_string()),
            (
                "project-id".to_string(),
                "8cf3ec0e-d0c2-4fcd-8206-ad204f254a96".to_string(),
            ),
            ("definition-id".to_string(), "1".to_string()),
            ("environment-id".to_string(), "1".to_string()),
        ])
    }

    fn badge_svg(value: &str) -> String {
        format!("<svg><g><text>release</text></g><g><text>{value}</text></g></svg>")
    }

    #[test]
    fn extracts_the_release_status_from_the_scraped_svg() {
        let fetcher = FakeFetcher {
            expected_url: "https://vsrm.dev.azure.com/totodem/_apis/public/Release/badge/8cf3ec0e-d0c2-4fcd-8206-ad204f254a96/1/1",
            body: badge_svg("succeeded"),
        };
        let value = resolve_release(&params(), &fetcher).unwrap();
        assert_eq!(value, "succeeded");
    }

    #[test]
    fn requires_all_four_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_release(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("environment-id".to_string(), String::new());
        assert!(resolve_release(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params();
        p.insert("organization".to_string(), "../etc".to_string());
        assert!(resolve_release(&p, &Unused).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://vsrm.dev.azure.com/totodem/_apis/public/Release/badge/8cf3ec0e-d0c2-4fcd-8206-ad204f254a96/1/1",
            body: "not an svg".to_string(),
        };
        assert!(resolve_release(&params(), &fetcher).is_err());
    }
}
