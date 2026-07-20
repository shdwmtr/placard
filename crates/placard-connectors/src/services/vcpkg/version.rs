use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let port_name = params
        .get("port-name")
        .ok_or("vcpkg-version requires a data-port-name attribute")?;
    let port_name = validate_path_param("port-name", port_name)?;

    let url = format!(
        "https://raw.githubusercontent.com/microsoft/vcpkg/master/ports/{port_name}/vcpkg.json"
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "vcpkg response was not valid UTF-8".to_string())?;
    let manifest = json::parse(&text)?;

    for field in [
        "version-date",
        "version-semver",
        "version-string",
        "version",
    ] {
        if let Some(value) = manifest.get(field) {
            if let Some(text) = value.as_text() {
                return Ok(text);
            }
        }
    }
    Err("vcpkg manifest missing a version field".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/microsoft/vcpkg/master/ports/entt/vcpkg.json"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(port_name: &str) -> HashMap<String, String> {
        HashMap::from([("port-name".to_string(), port_name.to_string())])
    }

    #[test]
    fn extracts_the_version_field() {
        let fetcher = FakeFetcher(r#"{"name": "entt", "version": "3.13.0"}"#);
        let value = resolve_version(&params("entt"), &fetcher).unwrap();
        assert_eq!(value, "3.13.0");
    }

    #[test]
    fn prefers_version_date_over_other_version_fields() {
        let fetcher =
            FakeFetcher(r#"{"name": "x", "version-date": "2024-01-01", "version": "1.0.0"}"#);
        let value = resolve_version(&params("entt"), &fetcher).unwrap();
        assert_eq!(value, "2024-01-01");
    }

    #[test]
    fn falls_back_to_version_semver_then_version_string() {
        let fetcher = FakeFetcher(r#"{"name": "x", "version-semver": "1.2.3"}"#);
        assert_eq!(resolve_version(&params("entt"), &fetcher).unwrap(), "1.2.3");

        let fetcher = FakeFetcher(r#"{"name": "x", "version-string": "abc123"}"#);
        assert_eq!(
            resolve_version(&params("entt"), &fetcher).unwrap(),
            "abc123"
        );
    }

    #[test]
    fn requires_a_port_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
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
        assert!(resolve_version(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_version_field_is_present() {
        let fetcher = FakeFetcher(r#"{"name": "entt"}"#);
        assert!(resolve_version(&params("entt"), &fetcher).is_err());
    }
}
