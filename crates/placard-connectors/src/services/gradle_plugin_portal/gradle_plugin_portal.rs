use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn extract_element(xml: &str, tag: &str) -> Option<String> {
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

/// Mirrors upstream's redirect to the `maven-metadata` badge: it builds the
/// plugin's `maven-metadata.xml` URL from its dotted id and reports the
/// `release` version (falling back to `latest`) the same way that badge does.
pub(crate) fn resolve_gradle_plugin_portal(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let plugin_id = params
        .get("plugin-id")
        .ok_or("gradle-plugin-portal requires a data-plugin-id attribute")?;
    let plugin_id = validate_path_param("plugin-id", plugin_id)?;

    let group_path = plugin_id.replace('.', "/");
    let artifact_id = format!("{plugin_id}.gradle.plugin");
    let url =
        format!("https://plugins.gradle.org/m2/{group_path}/{artifact_id}/maven-metadata.xml");

    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "gradle plugin portal response was not valid UTF-8".to_string())?;

    extract_element(&text, "release")
        .or_else(|| extract_element(&text, "latest"))
        .ok_or_else(|| "gradle plugin portal response missing release/latest version".to_string())
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

    fn params(plugin_id: &str) -> HashMap<String, String> {
        HashMap::from([("plugin-id".to_string(), plugin_id.to_string())])
    }

    #[test]
    fn extracts_the_release_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://plugins.gradle.org/m2/com/gradle/plugin-publish/com.gradle.plugin-publish.gradle.plugin/maven-metadata.xml",
            body: r#"<metadata><versioning><latest>1.2.1</latest><release>1.2.1</release></versioning></metadata>"#,
        };
        let value =
            resolve_gradle_plugin_portal(&params("com.gradle.plugin-publish"), &fetcher).unwrap();
        assert_eq!(value, "1.2.1");
    }

    #[test]
    fn falls_back_to_latest_when_release_is_absent() {
        let fetcher = FakeFetcher {
            expected_url: "https://plugins.gradle.org/m2/com/gradle/plugin-publish/com.gradle.plugin-publish.gradle.plugin/maven-metadata.xml",
            body: r#"<metadata><versioning><latest>1.3.0-beta</latest></versioning></metadata>"#,
        };
        let value =
            resolve_gradle_plugin_portal(&params("com.gradle.plugin-publish"), &fetcher).unwrap();
        assert_eq!(value, "1.3.0-beta");
    }

    #[test]
    fn requires_a_plugin_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid plugin-id")
            }
        }
        assert!(resolve_gradle_plugin_portal(&HashMap::new(), &Unused).is_err());
        assert!(resolve_gradle_plugin_portal(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_plugin_id() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid plugin-id")
            }
        }
        assert!(resolve_gradle_plugin_portal(&params("a/b"), &Unused).is_err());
    }

    #[test]
    fn errors_when_neither_release_nor_latest_is_present() {
        let fetcher = FakeFetcher {
            expected_url: "https://plugins.gradle.org/m2/com/example/plugin/com.example.plugin.gradle.plugin/maven-metadata.xml",
            body: r#"<metadata><versioning><versions><version>1.0</version></versions></versioning></metadata>"#,
        };
        assert!(resolve_gradle_plugin_portal(&params("com.example.plugin"), &fetcher).is_err());
    }
}
