use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let group_id = params
        .get("group-id")
        .ok_or("jitpack-version requires a data-group-id attribute")?;
    let artifact_id = params
        .get("artifact-id")
        .ok_or("jitpack-version requires a data-artifact-id attribute")?;
    let group_id = validate_path_param("group-id", group_id)?;
    let artifact_id = validate_path_param("artifact-id", artifact_id)?;

    let url = format!("https://jitpack.io/api/builds/{group_id}/{artifact_id}/latestOk");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "jitpack response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("jitpack response missing version")?;
    version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://jitpack.io/api/builds/com.github.jitpack/maven-simple/latestOk"
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
    fn extracts_version_from_a_jitpack_shaped_response() {
        let fetcher = FakeFetcher(r#"{"version": "1.0", "status": "ok"}"#);
        let value =
            resolve_version(&params("com.github.jitpack", "maven-simple"), &fetcher).unwrap();
        assert_eq!(value, "1.0");
    }

    #[test]
    fn requires_group_id_and_artifact_id_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params("com.github.jitpack", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc", "maven-simple"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"status": "ok"}"#);
        assert!(resolve_version(&params("com.github.jitpack", "maven-simple"), &fetcher).is_err());
    }
}
