use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

/// Travis build status badge SVGs render their value as the last
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

pub(crate) fn resolve_build(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("travis-build requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("travis-build requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let mut url = format!("https://api.travis-ci.com/{user}/{repo}.svg");
    if let Some(branch) = params.get("branch") {
        let branch = validate_path_param("branch", branch)?;
        url.push_str("?branch=");
        url.push_str(branch);
    }

    let bytes = fetcher.fetch(&url)?;
    let svg =
        String::from_utf8(bytes).map_err(|_| "travis response was not valid UTF-8".to_string())?;
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

    fn params(user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    fn badge_svg(value: &str) -> String {
        format!("<svg><g><text>build</text></g><g><text>{value}</text></g></svg>")
    }

    #[test]
    fn extracts_the_build_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.travis-ci.com/ivandelabeldad/rackian-gateway.svg",
            body: badge_svg("passing"),
        };
        let value = resolve_build(&params("ivandelabeldad", "rackian-gateway"), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn includes_the_branch_query_param_when_given() {
        let mut p = params("ivandelabeldad", "rackian-gateway");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://api.travis-ci.com/ivandelabeldad/rackian-gateway.svg?branch=master",
            body: badge_svg("failing"),
        };
        let value = resolve_build(&p, &fetcher).unwrap();
        assert_eq!(value, "failing");
    }

    #[test]
    fn requires_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_build(&HashMap::new(), &Unused).is_err());
        assert!(resolve_build(&params("ivandelabeldad", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_build(&params("../etc", "rackian-gateway"), &Unused).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.travis-ci.com/ivandelabeldad/rackian-gateway.svg",
            body: "not an svg".to_string(),
        };
        assert!(resolve_build(&params("ivandelabeldad", "rackian-gateway"), &fetcher).is_err());
    }
}
