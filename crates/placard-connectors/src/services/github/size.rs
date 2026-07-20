use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_segmented_param<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("'{name}' parameter must not be empty"));
    }
    for segment in value.split('/') {
        validate_path_param(name, segment)?;
    }
    Ok(value)
}

pub(crate) fn resolve_size(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-size requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-size requires a data-repo attribute")?;
    let path = params
        .get("path")
        .ok_or("github-size requires a data-path attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let path = validate_segmented_param("path", path)?;

    let mut url = format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}");
    if let Some(branch) = params.get("branch") {
        let branch = validate_path_param("branch", branch)?;
        url.push_str("?ref=");
        url.push_str(branch);
    }

    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    match value {
        Value::Object(_) => value
            .get("size")
            .ok_or("github response missing size")?
            .as_text()
            .ok_or_else(|| "size was not a plain value".to_string()),
        Value::Array(_) => Err("path is a directory, not a file".to_string()),
        _ => Err("github response was not a JSON object".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.github.com/repos/webcaetano/craft/contents/build/phaser-craft.min.js"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str, path: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
            ("path".to_string(), path.to_string()),
        ])
    }

    #[test]
    fn extracts_the_size_field_of_a_file() {
        let fetcher = FakeFetcher(r#"{"name": "phaser-craft.min.js", "size": 483920}"#);
        let value = resolve_size(
            &params("webcaetano", "craft", "build/phaser-craft.min.js"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "483920");
    }

    #[test]
    fn appends_ref_query_param_when_branch_given() {
        struct BranchFetcher;
        impl Fetcher for BranchFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://api.github.com/repos/webcaetano/craft/contents/build/phaser-craft.min.js?ref=master"
                );
                Ok(br#"{"size": 100}"#.to_vec())
            }
        }
        let mut p = params("webcaetano", "craft", "build/phaser-craft.min.js");
        p.insert("branch".to_string(), "master".to_string());
        let value = resolve_size(&p, &BranchFetcher).unwrap();
        assert_eq!(value, "100");
    }

    #[test]
    fn requires_owner_repo_and_path_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_size(&HashMap::new(), &Unused).is_err());
        assert!(resolve_size(&params("webcaetano", "craft", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_size(&params("../etc", "craft", "build/x.js"), &Unused).is_err());
        assert!(resolve_size(&params("webcaetano", "craft", "build/x.js?y=1"), &Unused).is_err());
    }

    #[test]
    fn errors_when_path_is_a_directory() {
        let fetcher = FakeFetcher(r#"[{"name": "a.js", "size": 1}, {"name": "b.js", "size": 2}]"#);
        assert!(
            resolve_size(
                &params("webcaetano", "craft", "build/phaser-craft.min.js"),
                &fetcher
            )
            .is_err()
        );
    }
}
