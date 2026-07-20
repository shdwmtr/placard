use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

/// CodeFactor grade badge SVGs render their value as the last
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

pub(crate) fn resolve_grade(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let vcs_type = params
        .get("vcs-type")
        .ok_or("codefactor-grade requires a data-vcs-type attribute")?;
    if !matches!(vcs_type.as_str(), "github" | "bitbucket") {
        return Err(format!(
            "'vcs-type' parameter '{vcs_type}' is not one of github, bitbucket"
        ));
    }
    let user = params
        .get("user")
        .ok_or("codefactor-grade requires a data-user attribute")?;
    let repo = params
        .get("repo")
        .ok_or("codefactor-grade requires a data-repo attribute")?;
    let user = validate_path_param("user", user)?;
    let repo = validate_path_param("repo", repo)?;

    let branch = match params.get("branch") {
        Some(branch) if !branch.is_empty() => validate_path_param("branch", branch)?,
        Some(_) => return Err("'branch' parameter must not be empty".to_string()),
        None => "",
    };

    let url = format!("https://codefactor.io/repository/{vcs_type}/{user}/{repo}/badge/{branch}");

    let bytes = fetcher.fetch(&url)?;
    let svg = String::from_utf8(bytes)
        .map_err(|_| "codefactor response was not valid UTF-8".to_string())?;
    let grade = extract_svg_text(&svg)?;

    if !matches!(
        grade.as_str(),
        "A+" | "A" | "A-" | "B+" | "B" | "B-" | "C+" | "C" | "C-" | "D+" | "D" | "D-" | "F" | "-"
    ) {
        return Err(format!("unexpected codefactor grade '{grade}'"));
    }
    Ok(grade)
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

    fn params(vcs_type: &str, user: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("vcs-type".to_string(), vcs_type.to_string()),
            ("user".to_string(), user.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    fn badge_svg(value: &str) -> String {
        format!("<svg><g><text>code quality</text></g><g><text>{value}</text></g></svg>")
    }

    #[test]
    fn extracts_the_grade() {
        let fetcher = FakeFetcher {
            expected_url: "https://codefactor.io/repository/github/microsoft/powertoys/badge/",
            body: badge_svg("A"),
        };
        let value = resolve_grade(&params("github", "microsoft", "powertoys"), &fetcher).unwrap();
        assert_eq!(value, "A");
    }

    #[test]
    fn includes_the_branch_path_segment_when_given() {
        let mut p = params("github", "microsoft", "powertoys");
        p.insert("branch".to_string(), "main".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://codefactor.io/repository/github/microsoft/powertoys/badge/main",
            body: badge_svg("A-"),
        };
        let value = resolve_grade(&p, &fetcher).unwrap();
        assert_eq!(value, "A-");
    }

    #[test]
    fn requires_vcs_type_user_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_grade(&HashMap::new(), &Unused).is_err());
        assert!(resolve_grade(&params("svn", "microsoft", "powertoys"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_grade(&params("github", "../etc", "powertoys"), &Unused).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://codefactor.io/repository/github/microsoft/powertoys/badge/",
            body: "not an svg".to_string(),
        };
        assert!(resolve_grade(&params("github", "microsoft", "powertoys"), &fetcher).is_err());
    }
}
