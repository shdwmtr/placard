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

fn validate_branch(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'branch' parameter must not be empty".to_string());
    }
    for segment in value.split('/') {
        if segment == "." || segment == ".." {
            return Err("'branch' parameter contains disallowed characters".to_string());
        }
        validate_path_param("branch", segment)?;
    }
    Ok(value)
}

/// Codecov's coverage badge SVGs render their value as the last
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

pub(crate) fn resolve_codecov(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let vcs_name = params
        .get("vcs-name")
        .ok_or("codecov requires a data-vcs-name attribute")?;
    if !matches!(
        vcs_name.as_str(),
        "github" | "gh" | "bitbucket" | "bb" | "gl" | "gitlab"
    ) {
        return Err(format!(
            "'vcs-name' parameter '{vcs_name}' is not one of github, gh, bitbucket, bb, gl, gitlab"
        ));
    }
    let user = params
        .get("user")
        .ok_or("codecov requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("codecov requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let mut url = format!("https://codecov.io/{vcs_name}/{user}/{repo}");
    if let Some(branch) = params.get("branch") {
        let branch = validate_branch(branch)?;
        url.push_str("/branch/");
        url.push_str(branch);
    }
    url.push_str("/graph/badge.svg");

    let mut query = Vec::new();
    if let Some(token) = params.get("token") {
        if !token.is_empty() {
            query.push(format!("token={}", percent_encode(token)));
        }
    }
    if let Some(flag) = params.get("flag") {
        if !flag.is_empty() {
            query.push(format!("flag={}", percent_encode(flag)));
        }
    }
    if let Some(component) = params.get("component") {
        if !component.is_empty() {
            query.push(format!("component={}", percent_encode(component)));
        }
    }
    if !query.is_empty() {
        url.push('?');
        url.push_str(&query.join("&"));
    }

    let bytes = fetcher.fetch(&url)?;
    let svg =
        String::from_utf8(bytes).map_err(|_| "codecov response was not valid UTF-8".to_string())?;
    let coverage = extract_svg_text(&svg)?;

    if coverage == "unknown" {
        return Ok(coverage);
    }
    if !coverage.ends_with('%')
        || !coverage[..coverage.len() - 1]
            .chars()
            .all(|c| c.is_ascii_digit())
    {
        return Err("unparseable coverage value".to_string());
    }
    Ok(coverage)
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

    fn params(vcs_name: &str, user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("vcs-name".to_string(), vcs_name.to_string()),
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    fn badge_svg(value: &str) -> String {
        format!("<svg><g><text>coverage</text></g><g><text>{value}</text></g></svg>")
    }

    #[test]
    fn extracts_the_coverage_percentage() {
        let fetcher = FakeFetcher {
            expected_url: "https://codecov.io/github/codecov/example-node/graph/badge.svg",
            body: badge_svg("92%"),
        };
        let value =
            resolve_codecov(&params("github", "codecov", "example-node"), &fetcher).unwrap();
        assert_eq!(value, "92%");
    }

    #[test]
    fn includes_the_branch_path_segment_when_given() {
        let mut p = params("github", "codecov", "example-node");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://codecov.io/github/codecov/example-node/branch/master/graph/badge.svg",
            body: badge_svg("50%"),
        };
        let value = resolve_codecov(&p, &fetcher).unwrap();
        assert_eq!(value, "50%");
    }

    #[test]
    fn includes_query_params_when_given() {
        let mut p = params("gh", "codecov", "example-node");
        p.insert("flag".to_string(), "unit_tests".to_string());
        p.insert("component".to_string(), "core".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://codecov.io/gh/codecov/example-node/graph/badge.svg?flag=unit_tests&component=core",
            body: badge_svg("75%"),
        };
        let value = resolve_codecov(&p, &fetcher).unwrap();
        assert_eq!(value, "75%");
    }

    #[test]
    fn returns_unknown_when_the_badge_reports_it() {
        let fetcher = FakeFetcher {
            expected_url: "https://codecov.io/github/codecov/example-node/graph/badge.svg",
            body: badge_svg("unknown"),
        };
        let value =
            resolve_codecov(&params("github", "codecov", "example-node"), &fetcher).unwrap();
        assert_eq!(value, "unknown");
    }

    #[test]
    fn requires_vcs_name_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_codecov(&HashMap::new(), &Unused).is_err());
        assert!(resolve_codecov(&params("svn", "codecov", "example-node"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_codecov(&params("github", "../etc", "example-node"), &Unused).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://codecov.io/github/codecov/example-node/graph/badge.svg",
            body: "not an svg".to_string(),
        };
        assert!(resolve_codecov(&params("github", "codecov", "example-node"), &fetcher).is_err());
    }
}
