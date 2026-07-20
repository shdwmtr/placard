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

/// Codacy grade badge SVGs render their value in a
/// `visibility="hidden">VALUE</text>` element.
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

pub(crate) fn resolve_grade(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_id = params
        .get("project-id")
        .ok_or("codacy-grade requires a data-project-id attribute")?;
    let project_id = validate_path_param("project-id", project_id)?;

    let mut url = format!("https://api.codacy.com/project/badge/grade/{project_id}");
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
    let grade = extract_marked_text(&svg, "visibility=\"hidden\">")?;

    if !matches!(grade.as_str(), "A" | "B" | "C" | "D" | "E" | "F") {
        return Err(format!("unexpected codacy grade '{grade}'"));
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

    fn params(project_id: &str) -> HashMap<String, String> {
        HashMap::from([("project-id".to_string(), project_id.to_string())])
    }

    fn badge_svg(value: &str) -> String {
        format!(r#"<svg><text x="50" visibility="hidden">{value}</text></svg>"#)
    }

    #[test]
    fn extracts_the_grade() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/grade/0cb32ce695b743d68257021455330c66",
            body: badge_svg("A"),
        };
        let value = resolve_grade(&params("0cb32ce695b743d68257021455330c66"), &fetcher).unwrap();
        assert_eq!(value, "A");
    }

    #[test]
    fn includes_the_branch_query_param_when_given() {
        let mut p = params("0cb32ce695b743d68257021455330c66");
        p.insert("branch".to_string(), "master".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/grade/0cb32ce695b743d68257021455330c66?branch=master",
            body: badge_svg("C"),
        };
        let value = resolve_grade(&p, &fetcher).unwrap();
        assert_eq!(value, "C");
    }

    #[test]
    fn requires_project_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project id")
            }
        }
        assert!(resolve_grade(&HashMap::new(), &Unused).is_err());
        assert!(resolve_grade(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project id")
            }
        }
        assert!(resolve_grade(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_on_an_unexpected_grade_value() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/grade/0cb32ce695b743d68257021455330c66",
            body: badge_svg("Z"),
        };
        assert!(resolve_grade(&params("0cb32ce695b743d68257021455330c66"), &fetcher).is_err());
    }

    #[test]
    fn errors_on_an_unparseable_svg_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.codacy.com/project/badge/grade/0cb32ce695b743d68257021455330c66",
            body: "not an svg".to_string(),
        };
        assert!(resolve_grade(&params("0cb32ce695b743d68257021455330c66"), &fetcher).is_err());
    }
}
