use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

const BASE_URL: &str = "https://toolshed.g2.bx.psu.edu";

pub(crate) fn resolve_downloads(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let repository = params
        .get("repository")
        .ok_or("galaxytoolshed-downloads requires a data-repository attribute")?;
    let owner = params
        .get("owner")
        .ok_or("galaxytoolshed-downloads requires a data-owner attribute")?;
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
    let revision_info = entries.first().ok_or("galaxytoolshed response was empty")?;
    revision_info
        .get("times_downloaded")
        .and_then(Value::as_text)
        .ok_or_else(|| "galaxytoolshed response missing times_downloaded".to_string())
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

    fn params() -> HashMap<String, String> {
        HashMap::from([
            ("repository".to_string(), "sra_tools".to_string()),
            ("owner".to_string(), "iuc".to_string()),
        ])
    }

    #[test]
    fn extracts_times_downloaded_from_the_latest_revision() {
        let fetcher = FakeFetcher {
            revisions_response: r#"["abc123", "def456"]"#,
            info_response: r#"[{"create_time": "2013-03-07T17:33:00", "times_downloaded": 4213}, {"changeset_revision": "abc123"}]"#,
            calls: AtomicUsize::new(0),
        };
        let value = resolve_downloads(&params(), &fetcher).unwrap();
        assert_eq!(value, "4213");
    }

    #[test]
    fn requires_repository_and_owner_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_downloads(&HashMap::new(), &Unused).is_err());
        let mut p = params();
        p.insert("repository".to_string(), String::new());
        assert!(resolve_downloads(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params();
        p.insert("repository".to_string(), "../etc".to_string());
        assert!(resolve_downloads(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_times_downloaded_field_is_missing() {
        let fetcher = FakeFetcher {
            revisions_response: r#"["abc123"]"#,
            info_response: r#"[{"create_time": "2013-03-07T17:33:00"}, {"changeset_revision": "abc123"}]"#,
            calls: AtomicUsize::new(0),
        };
        assert!(resolve_downloads(&params(), &fetcher).is_err());
    }
}
