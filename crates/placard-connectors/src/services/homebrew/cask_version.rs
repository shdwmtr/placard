use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_cask_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let cask = params
        .get("cask")
        .ok_or("homebrew-cask-version requires a data-cask attribute")?;
    let cask = validate_path_param("cask", cask)?;

    let url = format!("https://formulae.brew.sh/api/cask/{cask}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "homebrew response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("homebrew response missing version")?;
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
            assert_eq!(url, "https://formulae.brew.sh/api/cask/iterm2.json");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(cask: &str) -> HashMap<String, String> {
        HashMap::from([("cask".to_string(), cask.to_string())])
    }

    #[test]
    fn extracts_version_from_a_homebrew_cask_shaped_response() {
        let fetcher = FakeFetcher(r#"{"token": "iterm2", "version": "3.5.0"}"#);
        let value = resolve_cask_version(&params("iterm2"), &fetcher).unwrap();
        assert_eq!(value, "3.5.0");
    }

    #[test]
    fn requires_cask_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_cask_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_cask_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_cask_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"token": "iterm2"}"#);
        assert!(resolve_cask_version(&params("iterm2"), &fetcher).is_err());
    }
}
