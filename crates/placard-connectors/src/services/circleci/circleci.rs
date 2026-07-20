use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn vcs_segment(vcs_type: &str) -> Result<&'static str, String> {
    match vcs_type {
        "gh" | "github" => Ok("gh"),
        "bb" | "bitbucket" => Ok("bb"),
        other => Err(format!(
            "'vcs-type' parameter must be one of github, gh, bitbucket, bb, got '{other}'"
        )),
    }
}

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

/// CircleCI badge SVGs render their value as the last `<text>...</text></g>`
/// element. Mirrors shields' `BaseSvgScrapingService.valueFromSvgBadge`:
/// strip newlines (and any whitespace immediately following one), then take
/// the text content of the rightmost `</text></g>` element.
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

pub(crate) fn resolve_circleci(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let vcs_type = params
        .get("vcs-type")
        .ok_or("circleci requires a data-vcs-type attribute")?;
    let vcs = vcs_segment(vcs_type)?;
    let user = params
        .get("user")
        .ok_or("circleci requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = params
        .get("repo")
        .ok_or("circleci requires a data-repo attribute")?;
    let repo = validate_path_param("repo", repo)?;

    let mut url = format!("https://circleci.com/{vcs}/{user}/{repo}");
    if let Some(branch) = params.get("branch") {
        if !branch.is_empty() {
            let branch = validate_path_param("branch", branch)?;
            url.push_str("/tree/");
            url.push_str(branch);
        }
    }
    url.push_str(".svg?style=shield");
    if let Some(token) = params.get("token") {
        if !token.is_empty() {
            url.push_str("&circle-token=");
            url.push_str(&percent_encode(token));
        }
    }

    let bytes = fetcher.fetch(&url)?;
    let svg = String::from_utf8(bytes)
        .map_err(|_| "circleci response was not valid UTF-8".to_string())?;
    let status = extract_svg_text(&svg)?;
    Ok(status.replace('_', " "))
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

    fn params(
        vcs_type: &str,
        user: &str,
        repo: &str,
        branch: Option<&str>,
        token: Option<&str>,
    ) -> HashMap<String, String> {
        let mut map = HashMap::from([
            ("vcs-type".to_string(), vcs_type.to_string()),
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ]);
        if let Some(branch) = branch {
            map.insert("branch".to_string(), branch.to_string());
        }
        if let Some(token) = token {
            map.insert("token".to_string(), token.to_string());
        }
        map
    }

    const SVG_BODY: &str = "<svg><g><text>build</text><text>passed</text></g></svg>";

    #[test]
    fn extracts_status_from_an_svg_badge() {
        let fetcher = FakeFetcher {
            expected_url: "https://circleci.com/gh/RedSparr0w/node-csgo-parser.svg?style=shield",
            body: SVG_BODY,
        };
        let value = resolve_circleci(
            &params("github", "RedSparr0w", "node-csgo-parser", None, None),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "passed");
    }

    #[test]
    fn includes_branch_and_token_when_provided() {
        let fetcher = FakeFetcher {
            expected_url: "https://circleci.com/gh/RedSparr0w/node-csgo-parser/tree/master.svg?style=shield&circle-token=abc123",
            body: SVG_BODY,
        };
        let value = resolve_circleci(
            &params(
                "gh",
                "RedSparr0w",
                "node-csgo-parser",
                Some("master"),
                Some("abc123"),
            ),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "passed");
    }

    #[test]
    fn replaces_underscores_with_spaces() {
        let fetcher = FakeFetcher {
            expected_url: "https://circleci.com/bb/user/repo.svg?style=shield",
            body: "<svg><g><text>build</text><text>no_tests</text></g></svg>",
        };
        let value = resolve_circleci(&params("bb", "user", "repo", None, None), &fetcher).unwrap();
        assert_eq!(value, "no tests");
    }

    #[test]
    fn requires_vcs_type_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_circleci(&HashMap::new(), &Unused).is_err());
        assert!(resolve_circleci(&params("github", "", "repo", None, None), &Unused).is_err());
    }

    #[test]
    fn rejects_an_invalid_vcs_type() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid vcs-type")
            }
        }
        assert!(resolve_circleci(&params("svn", "user", "repo", None, None), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(
            resolve_circleci(&params("github", "../etc", "repo", None, None), &Unused).is_err()
        );
    }

    #[test]
    fn errors_on_an_unparseable_svg() {
        let fetcher = FakeFetcher {
            expected_url: "https://circleci.com/gh/user/repo.svg?style=shield",
            body: "<svg>not a badge</svg>",
        };
        assert!(resolve_circleci(&params("gh", "user", "repo", None, None), &fetcher).is_err());
    }
}
