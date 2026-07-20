use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_open_tickets(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project = params
        .get("project")
        .ok_or("sourceforge-open-tickets requires a data-project attribute")?;
    let project = validate_path_param("project", project)?;
    let ticket_type = match params.get("type").map(String::as_str) {
        Some(t @ ("bugs" | "feature-requests")) => t,
        Some(_) => {
            return Err(
                "sourceforge-open-tickets data-type must be 'bugs' or 'feature-requests'"
                    .to_string(),
            );
        }
        None => return Err("sourceforge-open-tickets requires a data-type attribute".to_string()),
    };

    let url = format!(
        "https://sourceforge.net/rest/p/{project}/{ticket_type}/search?limit=1&q=status%3Aopen"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "sourceforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("count")
        .ok_or("sourceforge response missing count")?;
    count
        .as_text()
        .ok_or_else(|| "count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://sourceforge.net/rest/p/sevenzip/bugs/search?limit=1&q=status%3Aopen"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(project: &str, ticket_type: &str) -> HashMap<String, String> {
        HashMap::from([
            ("project".to_string(), project.to_string()),
            ("type".to_string(), ticket_type.to_string()),
        ])
    }

    #[test]
    fn extracts_the_open_ticket_count() {
        let fetcher = FakeFetcher(r#"{"count": 42}"#);
        let value = resolve_open_tickets(&params("sevenzip", "bugs"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_project_and_type_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_open_tickets(&HashMap::new(), &Unused).is_err());
        assert!(resolve_open_tickets(&params("sevenzip", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_an_unknown_type() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid type")
            }
        }
        assert!(resolve_open_tickets(&params("sevenzip", "bogus"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_open_tickets(&params("../etc", "bugs"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_count_field_is_missing() {
        let fetcher = FakeFetcher(r#"{}"#);
        assert!(resolve_open_tickets(&params("sevenzip", "bugs"), &fetcher).is_err());
    }
}
