use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

/// Repology badge SVGs render their value as the last `<text>...</text></g>`
/// element, the same shape shields' generic SVG badge scraper reads.
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

pub(crate) fn resolve_repositories(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_name = params
        .get("project-name")
        .ok_or("repology-repositories requires a data-project-name attribute")?;
    let project_name = validate_path_param("project-name", project_name)?;

    let url = format!("https://repology.org/badge/tiny-repos/{project_name}.svg");
    let bytes = fetcher.fetch(&url)?;
    let svg = String::from_utf8(bytes)
        .map_err(|_| "repology response was not valid UTF-8".to_string())?;
    let count = extract_svg_text(&svg)?;
    if !count.chars().all(|c| c.is_ascii_digit()) || count.is_empty() {
        return Err(format!("unexpected repology-repositories value '{count}'"));
    }
    Ok(count)
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

    fn params(project_name: &str) -> HashMap<String, String> {
        HashMap::from([("project-name".to_string(), project_name.to_string())])
    }

    fn sample_svg(value: &str) -> String {
        format!(r#"<svg><g><text>repositories</text></g><g><text>{value}</text></g></svg>"#)
    }

    #[test]
    fn extracts_the_repository_count_from_a_repology_badge_svg() {
        let body = sample_svg("14");
        let fetcher = FakeFetcher {
            expected_url: "https://repology.org/badge/tiny-repos/starship.svg",
            body: Box::leak(body.into_boxed_str()),
        };
        let value = resolve_repositories(&params("starship"), &fetcher).unwrap();
        assert_eq!(value, "14");
    }

    #[test]
    fn requires_project_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_repositories(&HashMap::new(), &Unused).is_err());
        assert!(resolve_repositories(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_repositories(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_on_a_non_numeric_or_unparseable_svg() {
        let fetcher = FakeFetcher {
            expected_url: "https://repology.org/badge/tiny-repos/starship.svg",
            body: "<svg><g><text>not-svg</text></svg>",
        };
        assert!(resolve_repositories(&params("starship"), &fetcher).is_err());
    }
}
