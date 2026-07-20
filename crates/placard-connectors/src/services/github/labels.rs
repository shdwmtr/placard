use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn percent_encode_segment(input: &str) -> String {
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

pub(crate) fn resolve_labels(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-labels requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-labels requires a data-repo attribute")?;
    let name = params
        .get("name")
        .ok_or("github-labels requires a data-name attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    if name.is_empty() {
        return Err("'name' parameter must not be empty".to_string());
    }

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/labels/{}",
        percent_encode_segment(name)
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let name_field = value.get("name").ok_or("github response missing name")?;
    name_field
        .as_text()
        .ok_or_else(|| "name was not a plain value".to_string())
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

    fn params(owner: &str, repo: &str, name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("name".to_string(), name.to_string()),
        ])
    }

    #[test]
    fn extracts_the_label_name_from_a_github_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.github.com/repos/atom/atom/labels/help-wanted",
            body: r#"{"id": 1, "name": "help-wanted", "color": "159818"}"#,
        };
        let value = resolve_labels(&params("atom", "atom", "help-wanted"), &fetcher).unwrap();
        assert_eq!(value, "help-wanted");
    }

    #[test]
    fn percent_encodes_a_label_name_with_spaces() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.github.com/repos/atom/atom/labels/help%20wanted",
            body: r#"{"id": 1, "name": "help wanted", "color": "159818"}"#,
        };
        let value = resolve_labels(&params("atom", "atom", "help wanted"), &fetcher).unwrap();
        assert_eq!(value, "help wanted");
    }

    #[test]
    fn requires_owner_repo_and_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_labels(&HashMap::new(), &Unused).is_err());
        assert!(resolve_labels(&params("atom", "atom", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_labels(&params("../etc", "atom", "help-wanted"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.github.com/repos/atom/atom/labels/help-wanted",
            body: r#"{"id": 1, "color": "159818"}"#,
        };
        assert!(resolve_labels(&params("atom", "atom", "help-wanted"), &fetcher).is_err());
    }
}
