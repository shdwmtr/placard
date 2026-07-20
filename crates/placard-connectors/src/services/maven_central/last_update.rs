use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn extract_tag_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let text = xml[start..end].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

pub(crate) fn resolve_last_update(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let group_id = params
        .get("group-id")
        .ok_or("maven-central-last-update requires a data-group-id attribute")?;
    let artifact_id = params
        .get("artifact-id")
        .ok_or("maven-central-last-update requires a data-artifact-id attribute")?;
    let group_id = validate_path_param("group-id", group_id)?;
    let artifact_id = validate_path_param("artifact-id", artifact_id)?;
    let group_path = group_id.replace('.', "/");

    let url =
        format!("https://repo1.maven.org/maven2/{group_path}/{artifact_id}/maven-metadata.xml");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "maven-central response was not valid UTF-8".to_string())?;
    extract_tag_text(&text, "lastUpdated")
        .ok_or("maven-central response missing lastUpdated".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://repo1.maven.org/maven2/com/google/guava/guava/maven-metadata.xml"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(group_id: &str, artifact_id: &str) -> HashMap<String, String> {
        HashMap::from([
            ("group-id".to_string(), group_id.to_string()),
            ("artifact-id".to_string(), artifact_id.to_string()),
        ])
    }

    #[test]
    fn extracts_last_updated_from_a_maven_metadata_response() {
        let fetcher = FakeFetcher(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>com.google.guava</groupId>
  <artifactId>guava</artifactId>
  <versioning>
    <latest>33.0.0-jre</latest>
    <release>33.0.0-jre</release>
    <versions>
      <version>32.0.0-jre</version>
      <version>33.0.0-jre</version>
    </versions>
    <lastUpdated>20231211194529</lastUpdated>
  </versioning>
</metadata>"#,
        );
        let value = resolve_last_update(&params("com.google.guava", "guava"), &fetcher).unwrap();
        assert_eq!(value, "20231211194529");
    }

    #[test]
    fn requires_group_id_and_artifact_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_last_update(&HashMap::new(), &Unused).is_err());
        assert!(resolve_last_update(&params("com.google.guava", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_last_update(&params("../etc", "guava"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(
            r#"<?xml version="1.0" encoding="UTF-8"?><metadata><groupId>com.google.guava</groupId></metadata>"#,
        );
        assert!(resolve_last_update(&params("com.google.guava", "guava"), &fetcher).is_err());
    }
}
