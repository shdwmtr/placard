use crate::Fetcher;
use std::collections::HashMap;

fn validate_project(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'project' parameter must not be empty".to_string());
    }
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err("'project' parameter contains disallowed characters".to_string());
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err("'project' parameter contains disallowed characters".to_string());
        }
    }
    Ok(value)
}

fn validate_branch(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'branch' parameter must not be empty".to_string());
    }
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err("'branch' parameter contains disallowed characters".to_string());
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err("'branch' parameter contains disallowed characters".to_string());
        }
    }
    Ok(value)
}

fn resolve_base_url(params: &HashMap<String, String>) -> Result<String, String> {
    match params.get("gitlab-url") {
        Some(url) => {
            let trimmed = url.trim_end_matches('/');
            if trimmed.is_empty() {
                return Err("'gitlab-url' parameter must not be empty".to_string());
            }
            if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
                return Err("'gitlab-url' parameter must be an http(s) URL".to_string());
            }
            if trimmed
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
            {
                return Err("'gitlab-url' parameter contains disallowed characters".to_string());
            }
            Ok(trimmed.to_string())
        }
        None => Ok("https://gitlab.com".to_string()),
    }
}

/// GitLab badge SVGs render their value as the last `<text>...</text></g>`
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

pub(crate) fn resolve_pipeline_status(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("gitlab-pipeline-status requires a data-project attribute")?;
    let project = validate_project(project)?;
    let base_url = resolve_base_url(params)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_branch(branch)?,
        None => "main",
    };

    let url = format!("{base_url}/{project}/badges/{branch}/pipeline.svg");
    let bytes = fetcher.fetch(&url)?;
    let svg =
        String::from_utf8(bytes).map_err(|_| "gitlab response was not valid UTF-8".to_string())?;
    let status = extract_svg_text(&svg)?;
    if status == "unknown" {
        return Err("branch not found".to_string());
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
        format!("<svg><g><text>build</text></g><g><text>{value}</text></g></svg>")
    }

    #[test]
    fn extracts_the_pipeline_status_from_the_scraped_svg() {
        let body = badge_svg("passed");
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/gitlab-org/gitlab/badges/main/pipeline.svg",
            body: body.clone(),
        };
        let value = resolve_pipeline_status(&params("gitlab-org/gitlab"), &fetcher).unwrap();
        assert_eq!(value, "passed");
    }

    #[test]
    fn uses_the_provided_branch_and_gitlab_url() {
        let mut p = params("gitlab-org/gitlab");
        p.insert("branch".to_string(), "develop".to_string());
        p.insert(
            "gitlab-url".to_string(),
            "https://gitlab.example.com".to_string(),
        );
        let body = badge_svg("failed");
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.example.com/gitlab-org/gitlab/badges/develop/pipeline.svg",
            body: body.clone(),
        };
        let value = resolve_pipeline_status(&p, &fetcher).unwrap();
        assert_eq!(value, "failed");
    }

    #[test]
    fn requires_project_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project param")
            }
        }
        assert!(resolve_pipeline_status(&HashMap::new(), &Unused).is_err());
        assert!(resolve_pipeline_status(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_pipeline_status(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_branch_is_unknown() {
        let body = badge_svg("unknown");
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/gitlab-org/gitlab/badges/main/pipeline.svg",
            body: body.clone(),
        };
        assert!(resolve_pipeline_status(&params("gitlab-org/gitlab"), &fetcher).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://gitlab.com/gitlab-org/gitlab/badges/main/pipeline.svg",
            body: "not an svg".to_string(),
        };
        assert!(resolve_pipeline_status(&params("gitlab-org/gitlab"), &fetcher).is_err());
    }
}
