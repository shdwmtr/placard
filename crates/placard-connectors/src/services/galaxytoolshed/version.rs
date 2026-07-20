use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

const BASE_URL: &str = "https://toolshed.g2.bx.psu.edu";

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let repository = params
        .get("repository")
        .ok_or("galaxytoolshed-version requires a data-repository attribute")?;
    let owner = params
        .get("owner")
        .ok_or("galaxytoolshed-version requires a data-owner attribute")?;
    let repository = validate_path_param("repository", repository)?;
    let owner = validate_path_param("owner", owner)?;

    let revisions_url = format!(
        "{BASE_URL}/api/repositories/get_ordered_installable_revisions?name={repository}&owner={owner}"
    );
    let bytes = fetcher.fetch(&revisions_url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "galaxytoolshed response was not valid UTF-8".to_string())?;
    let revisions = json::parse(&text)?;
    let revisions = match &revisions {
        Value::Array(items) => items,
        _ => return Err("galaxytoolshed response was not an array".to_string()),
    };
    let changeset_revision = revisions
        .first()
        .and_then(Value::as_text)
        .ok_or("galaxytoolshed changesetRevision not found")?;
    let changeset_revision = validate_path_param("changeset_revision", &changeset_revision)?;

    let info_url = format!(
        "{BASE_URL}/api/repositories/get_repository_revision_install_info?name={repository}&owner={owner}&changeset_revision={changeset_revision}"
    );
    let bytes = fetcher.fetch(&info_url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "galaxytoolshed response was not valid UTF-8".to_string())?;
    let info = json::parse(&text)?;
    let entries = match &info {
        Value::Array(items) => items,
        _ => return Err("galaxytoolshed response was not an array".to_string()),
    };
    let revision_details = entries
        .get(1)
        .ok_or("galaxytoolshed response missing revision details")?;

    if let Some(tool_id) = params.get("tool") {
        let valid_tools = match revision_details.get("valid_tools") {
            Some(Value::Array(items)) => items,
            _ => return Err("galaxytoolshed response missing valid_tools".to_string()),
        };
        let tool = valid_tools
            .iter()
            .find(|t| t.get("id").and_then(Value::as_text).as_deref() == Some(tool_id.as_str()))
            .ok_or("tool not found")?;

        if let Some(requirement_id) = params.get("requirement") {
            let requirements = match tool.get("requirements") {
                Some(Value::Array(items)) => items,
                _ => return Err("galaxytoolshed tool missing requirements".to_string()),
            };
            let requirement = requirements
                .iter()
                .find(|r| {
                    r.get("name").and_then(Value::as_text).as_deref()
                        == Some(requirement_id.as_str())
                })
                .ok_or("requirement not found")?;
            return requirement
                .get("version")
                .and_then(Value::as_text)
                .ok_or_else(|| "galaxytoolshed requirement missing version".to_string());
        }

        return tool
            .get("version")
            .and_then(Value::as_text)
            .ok_or_else(|| "galaxytoolshed tool missing version".to_string());
    }

    revision_details
        .get("changeset_revision")
        .and_then(Value::as_text)
        .ok_or_else(|| "galaxytoolshed response missing changeset_revision".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeFetcher {
        revisions_response: &'static str,
        info_response: &'static str,
        calls: AtomicUsize,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => {
                    assert_eq!(
                        url,
                        "https://toolshed.g2.bx.psu.edu/api/repositories/get_ordered_installable_revisions?name=sra_tools&owner=iuc"
                    );
                    Ok(self.revisions_response.as_bytes().to_vec())
                }
                1 => {
                    assert_eq!(
                        url,
                        "https://toolshed.g2.bx.psu.edu/api/repositories/get_repository_revision_install_info?name=sra_tools&owner=iuc&changeset_revision=abc123"
                    );
                    Ok(self.info_response.as_bytes().to_vec())
                }
                _ => panic!("unexpected extra fetch"),
            }
        }
    }

    const INFO_RESPONSE: &str = r#"[
        {"create_time": "2013-03-07T17:33:00", "times_downloaded": 42},
        {
            "changeset_revision": "abc123",
            "valid_tools": [
                {
                    "id": "fastq_dump",
                    "version": "v2.11.0+galaxy1",
                    "requirements": [
                        {"name": "perl", "version": "5.32.1"},
                        {"name": "sra-tools", "version": "2.11.0"}
                    ]
                }
            ]
        }
    ]"#;

    fn params(tool: Option<&str>, requirement: Option<&str>) -> HashMap<String, String> {
        let mut m = HashMap::from([
            ("repository".to_string(), "sra_tools".to_string()),
            ("owner".to_string(), "iuc".to_string()),
        ]);
        if let Some(t) = tool {
            m.insert("tool".to_string(), t.to_string());
        }
        if let Some(r) = requirement {
            m.insert("requirement".to_string(), r.to_string());
        }
        m
    }

    #[test]
    fn extracts_repository_changeset_revision_by_default() {
        let fetcher = FakeFetcher {
            revisions_response: r#"["abc123", "def456"]"#,
            info_response: INFO_RESPONSE,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_version(&params(None, None), &fetcher).unwrap();
        assert_eq!(value, "abc123");
    }

    #[test]
    fn extracts_tool_version_when_tool_param_given() {
        let fetcher = FakeFetcher {
            revisions_response: r#"["abc123", "def456"]"#,
            info_response: INFO_RESPONSE,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_version(&params(Some("fastq_dump"), None), &fetcher).unwrap();
        assert_eq!(value, "v2.11.0+galaxy1");
    }

    #[test]
    fn extracts_requirement_version_when_both_params_given() {
        let fetcher = FakeFetcher {
            revisions_response: r#"["abc123", "def456"]"#,
            info_response: INFO_RESPONSE,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_version(&params(Some("fastq_dump"), Some("perl")), &fetcher).unwrap();
        assert_eq!(value, "5.32.1");
    }

    #[test]
    fn requires_repository_and_owner_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        let mut p = params(None, None);
        p.insert("owner".to_string(), String::new());
        assert!(resolve_version(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params(None, None);
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_version(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_tool_is_not_found() {
        let fetcher = FakeFetcher {
            revisions_response: r#"["abc123"]"#,
            info_response: INFO_RESPONSE,
            calls: AtomicUsize::new(0),
        };
        assert!(resolve_version(&params(Some("nonexistent"), None), &fetcher).is_err());
    }

    #[test]
    fn errors_when_requirement_is_not_found() {
        let fetcher = FakeFetcher {
            revisions_response: r#"["abc123"]"#,
            info_response: INFO_RESPONSE,
            calls: AtomicUsize::new(0),
        };
        assert!(
            resolve_version(&params(Some("fastq_dump"), Some("nonexistent")), &fetcher).is_err()
        );
    }
}
