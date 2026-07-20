use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let candidate = params
        .get("candidate")
        .ok_or("sdkman-version requires a data-candidate attribute")?;
    let candidate = validate_path_param("candidate", candidate)?;

    let url = format!("https://api.sdkman.io/2/candidates/default/{candidate}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "sdkman response was not valid UTF-8".to_string())?;
    let version = text.trim();
    if version.is_empty() {
        return Err("sdkman response was empty".to_string());
    }
    Ok(version.to_string())
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

    fn params(candidate: &str) -> HashMap<String, String> {
        HashMap::from([("candidate".to_string(), candidate.to_string())])
    }

    #[test]
    fn extracts_the_trimmed_default_version() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.sdkman.io/2/candidates/default/java",
            body: "  21.0.1-tem\n",
        };
        let value = resolve_version(&params("java"), &fetcher).unwrap();
        assert_eq!(value, "21.0.1-tem");
    }

    #[test]
    fn requires_candidate_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_response_body_is_empty() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.sdkman.io/2/candidates/default/java",
            body: "   ",
        };
        assert!(resolve_version(&params("java"), &fetcher).is_err());
    }
}
