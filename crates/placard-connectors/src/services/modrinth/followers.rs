use super::super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_followers(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_id = params
        .get("project-id")
        .ok_or("modrinth-followers requires a data-project-id attribute")?;
    let project_id = validate_path_param("project-id", project_id)?;

    let url = format!("https://api.modrinth.com/v2/project/{project_id}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "modrinth response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    value
        .get("followers")
        .ok_or("modrinth response missing followers")?
        .as_text()
        .ok_or_else(|| "followers was not a plain value".to_string())
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

    fn params(project_id: &str) -> HashMap<String, String> {
        HashMap::from([("project-id".to_string(), project_id.to_string())])
    }

    #[test]
    fn extracts_followers_from_a_modrinth_shaped_response() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.modrinth.com/v2/project/AANobbMI",
            body: r#"{"downloads": 123456, "followers": 42}"#,
        };
        let value = resolve_followers(&params("AANobbMI"), &fetcher).unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn requires_a_project_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project id")
            }
        }
        assert!(resolve_followers(&HashMap::new(), &Unused).is_err());
        assert!(resolve_followers(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_followers(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.modrinth.com/v2/project/AANobbMI",
            body: r#"{"downloads": 123456}"#,
        };
        assert!(resolve_followers(&params("AANobbMI"), &fetcher).is_err());
    }
}
