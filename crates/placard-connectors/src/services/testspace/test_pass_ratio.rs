use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_org(org: &str) -> Result<&str, String> {
    if org.is_empty() {
        return Err("'org' parameter must not be empty".to_string());
    }
    if !org
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("'org' parameter contains disallowed characters".to_string());
    }
    Ok(org)
}

fn validate_project(project: &str) -> Result<&str, String> {
    if project.is_empty() {
        return Err("'project' parameter must not be empty".to_string());
    }
    if !project
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
    {
        return Err("'project' parameter contains disallowed characters".to_string());
    }
    Ok(project)
}

fn validate_space(space: &str) -> Result<&str, String> {
    if space.is_empty() {
        return Err("'space' parameter must not be empty".to_string());
    }
    if !space
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    {
        return Err("'space' parameter contains disallowed characters".to_string());
    }
    Ok(space)
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

fn extract_case_counts(value: &json::Value) -> Result<[i64; 5], String> {
    let json::Value::Array(items) = value else {
        return Err("testspace response was not an array".to_string());
    };
    let first = items
        .first()
        .ok_or("testspace space not found or results purged")?;
    let counts = first
        .get("case_counts")
        .ok_or("testspace response missing case_counts")?;
    let json::Value::Array(counts) = counts else {
        return Err("testspace case_counts was not an array".to_string());
    };
    if counts.len() != 5 {
        return Err("testspace case_counts did not have 5 entries".to_string());
    }
    let mut out = [0i64; 5];
    for (i, v) in counts.iter().enumerate() {
        let text = v
            .as_text()
            .ok_or("testspace case_counts entry was not numeric")?;
        out[i] = text
            .parse::<i64>()
            .map_err(|_| "testspace case_counts entry was not numeric".to_string())?;
    }
    Ok(out)
}

fn build_url(org: &str, project: &str, space: &str) -> String {
    format!(
        "https://{org}.testspace.com/api/projects/{}/spaces/{space}/results",
        percent_encode(project)
    )
}

pub(crate) fn resolve_test_pass_ratio(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let org = params
        .get("org")
        .ok_or("testspace-test-pass-ratio requires a data-org attribute")?;
    let org = validate_org(org)?;
    let project = params
        .get("project")
        .ok_or("testspace-test-pass-ratio requires a data-project attribute")?;
    let project = validate_project(project)?;
    let space = params
        .get("space")
        .ok_or("testspace-test-pass-ratio requires a data-space attribute")?;
    let space = validate_space(space)?;

    let url = build_url(org, project, space);
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "testspace response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let [passed, failed, _skipped, errored, _untested] = extract_case_counts(&value)?;
    let total = passed + failed + errored;
    if total == 0 {
        return Err(
            "testspace-test-pass-ratio: no completed tests to compute a ratio from".to_string(),
        );
    }

    let ratio = (passed as f64 / total as f64 * 100.0).round() as i64;
    Ok(format!("{ratio}%"))
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

    fn params(org: &str, project: &str, space: &str) -> HashMap<String, String> {
        HashMap::from([
            ("org".to_string(), org.to_string()),
            ("project".to_string(), project.to_string()),
            ("space".to_string(), space.to_string()),
        ])
    }

    #[test]
    fn computes_pass_ratio_from_case_counts() {
        let fetcher = FakeFetcher {
            expected_url: "https://swellaby.testspace.com/api/projects/swellaby%3Atestspace-sample/spaces/main/results",
            body: r#"[{"case_counts": [90, 10, 0, 0, 0]}]"#,
        };
        let value = resolve_test_pass_ratio(
            &params("swellaby", "swellaby:testspace-sample", "main"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "90%");
    }

    #[test]
    fn requires_all_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_test_pass_ratio(&HashMap::new(), &Unused).is_err());
        assert!(resolve_test_pass_ratio(&params("", "proj", "space"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_test_pass_ratio(&params("org", "proj?x=1", "space"), &Unused).is_err());
    }

    #[test]
    fn errors_when_there_are_no_completed_tests() {
        let fetcher = FakeFetcher {
            expected_url: "https://swellaby.testspace.com/api/projects/proj/spaces/main/results",
            body: r#"[{"case_counts": [0, 0, 5, 0, 2]}]"#,
        };
        assert!(resolve_test_pass_ratio(&params("swellaby", "proj", "main"), &fetcher).is_err());
    }
}
