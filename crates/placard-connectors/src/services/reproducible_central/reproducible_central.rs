use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn find_version_result<'a>(value: &'a Value, version: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.iter().find(|(k, _)| k == version).map(|(_, v)| v),
        _ => None,
    }
}

pub(crate) fn resolve_reproducible_central(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let group_id = params
        .get("group-id")
        .ok_or("reproducible-central requires a data-group-id attribute")?;
    let artifact_id = params
        .get("artifact-id")
        .ok_or("reproducible-central requires a data-artifact-id attribute")?;
    let version = params
        .get("version")
        .ok_or("reproducible-central requires a data-version attribute")?;
    let group_id = validate_path_param("group-id", group_id)?;
    let artifact_id = validate_path_param("artifact-id", artifact_id)?;
    let version = validate_path_param("version", version)?;
    let group_path = group_id.replace('.', "/");

    let url = format!(
        "https://jvm-repo-rebuild.github.io/reproducible-central/badge/artifact/{group_path}/{artifact_id}.json"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "reproducible-central response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let result = find_version_result(&value, version)
        .ok_or("version not available in Maven Central".to_string())?;
    result
        .as_text()
        .ok_or_else(|| "reproducible-central result was not a plain value".to_string())
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

    fn params(group_id: &str, artifact_id: &str, version: &str) -> HashMap<String, String> {
        HashMap::from([
            ("group-id".to_string(), group_id.to_string()),
            ("artifact-id".to_string(), artifact_id.to_string()),
            ("version".to_string(), version.to_string()),
        ])
    }

    #[test]
    fn extracts_the_rebuild_result_for_the_requested_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://jvm-repo-rebuild.github.io/reproducible-central/badge/artifact/org/apache/maven/maven-core.json",
            body: r#"{"3.9.9": "5/5", "3.9.8": "X"}"#,
        };
        let value = resolve_reproducible_central(
            &params("org.apache.maven", "maven-core", "3.9.9"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "5/5");
    }

    #[test]
    fn extracts_a_non_numeric_status_for_the_requested_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://jvm-repo-rebuild.github.io/reproducible-central/badge/artifact/org/apache/maven/maven-core.json",
            body: r#"{"3.9.8": "X"}"#,
        };
        let value = resolve_reproducible_central(
            &params("org.apache.maven", "maven-core", "3.9.8"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "X");
    }

    #[test]
    fn requires_group_id_artifact_id_and_version_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_reproducible_central(&HashMap::new(), &Unused).is_err());
        assert!(
            resolve_reproducible_central(&params("org.apache.maven", "maven-core", ""), &Unused)
                .is_err()
        );
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
            resolve_reproducible_central(&params("../etc", "maven-core", "3.9.9"), &Unused)
                .is_err()
        );
    }

    #[test]
    fn errors_when_the_version_is_not_present() {
        let fetcher = FakeFetcher {
            expected_url: "https://jvm-repo-rebuild.github.io/reproducible-central/badge/artifact/org/apache/maven/maven-core.json",
            body: r#"{"3.9.8": "X"}"#,
        };
        assert!(
            resolve_reproducible_central(
                &params("org.apache.maven", "maven-core", "9.9.9"),
                &fetcher
            )
            .is_err()
        );
    }
}
