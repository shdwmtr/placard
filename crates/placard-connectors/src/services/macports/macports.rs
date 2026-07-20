use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_macports(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let port_name = params
        .get("port-name")
        .ok_or("macports requires a data-port-name attribute")?;
    let port_name = validate_path_param("port-name", port_name)?;

    let url = format!("https://ports.macports.org/api/v1/ports/{port_name}/");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "macports response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("macports response missing version")?;
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
            assert_eq!(url, "https://ports.macports.org/api/v1/ports/git/");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(port_name: &str) -> HashMap<String, String> {
        HashMap::from([("port-name".to_string(), port_name.to_string())])
    }

    #[test]
    fn extracts_the_version_field() {
        let fetcher = FakeFetcher(r#"{"name": "git", "version": "2.43.0"}"#);
        let value = resolve_macports(&params("git"), &fetcher).unwrap();
        assert_eq!(value, "2.43.0");
    }

    #[test]
    fn requires_port_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid port name")
            }
        }
        assert!(resolve_macports(&HashMap::new(), &Unused).is_err());
        assert!(resolve_macports(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid port name")
            }
        }
        assert!(resolve_macports(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_version_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "git"}"#);
        assert!(resolve_macports(&params("git"), &fetcher).is_err());
    }
}
