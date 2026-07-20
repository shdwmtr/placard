use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_elm_package(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("elm-package requires a data-user attribute")?;
    let user = validate_path_param("user", user)?;
    let package = params
        .get("package")
        .ok_or("elm-package requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://package.elm-lang.org/packages/{user}/{package}/latest/elm.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "elm package response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let version = value
        .get("version")
        .ok_or("elm package response missing version")?;
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
                "https://package.elm-lang.org/packages/elm/core/latest/elm.json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, package: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("package".to_string(), package.to_string()),
        ])
    }

    #[test]
    fn extracts_version_from_an_elm_json_shaped_response() {
        let fetcher = FakeFetcher(r#"{"version": "1.0.5", "name": "elm/core"}"#);
        let value = resolve_elm_package(&params("elm", "core"), &fetcher).unwrap();
        assert_eq!(value, "1.0.5");
    }

    #[test]
    fn requires_user_and_package_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_elm_package(&HashMap::new(), &Unused).is_err());
        assert!(resolve_elm_package(&params("elm", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_elm_package(&params("../etc", "core"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"name": "elm/core"}"#);
        assert!(resolve_elm_package(&params("elm", "core"), &fetcher).is_err());
    }
}
